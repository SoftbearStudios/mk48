// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The game server has authority over all game logic. Clients are served the client, which connects
//! via websocket.

use crate::admin::ParameterizedAdminRequest;
use crate::client::Authenticate;
use crate::game_service::GameArenaService;
use crate::infrastructure::Infrastructure;
use crate::status::StatusRequest;
use crate::system::{SystemRepo, SystemRequest};
use actix::{fut, Actor, Addr};
use actix_cors::Cors;
use actix_web::body::{BodySize, MessageBody};
use actix_web::dev::{Service, ServiceResponse, Url};
use actix_web::http::header::{HeaderValue, ACCEPT, CACHE_CONTROL, LOCATION};
use actix_web::http::uri::PathAndQuery;
use actix_web::http::{header, Method, StatusCode, Uri};
use actix_web::web::{get, post, resource, Json, Query};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use common_util::ticks::Ticks;
use core_protocol::id::*;
use core_protocol::rpc::{Request, SystemQuery, Update, WebSocketQuery};
use futures::TryFutureExt;
use log::LevelFilter;
use log::{debug, warn};
use server_util::app::{game_static_files_hash, static_files};
use server_util::cloud::Cloud;
use server_util::ip_rate_limiter::IpRateLimiter;
use server_util::linode::Linode;
use server_util::rate_limiter::RateLimiterProps;
use server_util::ssl::{run_until_ssl_renewal, Ssl};
use server_util::tcp::{
    max_connections_per_worker, on_connect_enable_nodelay, BACKLOG, KEEP_ALIVE, SHUTDOWN_TIMEOUT,
};
use server_util::user_agent::UserAgent;
use server_util::web_socket::WebSocket;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use structopt::StructOpt;

/// Server options, to be specified as arguments.
#[derive(Debug, StructOpt)]
pub struct Options {
    /// Minimum player count (to be achieved by adding bots)
    #[structopt(short = "p", long, default_value = "30")]
    pub min_players: usize,
    /// Log incoming HTTP requests
    #[cfg_attr(debug_assertions, structopt(long, default_value = "warn"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    pub debug_http: LevelFilter,
    /// Log game diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "info"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    pub debug_game: LevelFilter,
    /// Log core diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "info"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    pub debug_core: LevelFilter,
    /// Log socket diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "warn"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    pub debug_sockets: LevelFilter,
    /// Log watchdog diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "info"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "warn"))]
    pub debug_watchdog: LevelFilter,
    /// Log chats here
    #[structopt(long)]
    pub chat_log: Option<String>,
    /// Log client traces here
    #[structopt(long)]
    pub trace_log: Option<String>,
    /// Linode personal access token for DNS configuration.
    #[structopt(long)]
    pub linode_personal_access_token: Option<String>,
    /// Don't write to the database.
    #[structopt(long)]
    pub database_read_only: bool,
    /// Server id.
    #[structopt(long, default_value = "0")]
    pub server_id: u8,
    #[structopt(long)]
    /// Override the server ip (currently used to detect the region).
    pub ip_address: Option<IpAddr>,
    /// Override the region id.
    #[structopt(long)]
    pub region_id: Option<RegionId>,
    /// Domain (without server id prepended).
    #[allow(dead_code)]
    #[structopt(long)]
    pub domain: Option<String>,
    // Certificate chain path
    #[structopt(long)]
    pub certificate_path: Option<String>,
    // Private key path
    #[structopt(long)]
    pub private_key_path: Option<String>,
}

/// 0 is no redirect.
static REDIRECT_TO_SERVER_ID: AtomicU8 = AtomicU8::new(0);

lazy_static::lazy_static! {
    static ref HTTP_RATE_LIMITER: Mutex<IpRateLimiter> = Mutex::new(IpRateLimiter::new_bandwidth_limiter(500_000, 10_000_000));
}

/// ws_index routes incoming HTTP requests to WebSocket connections.
async fn ws_index<G: GameArenaService>(
    r: HttpRequest,
    stream: web::Payload,
    query: WebSocketQuery,
    srv: Addr<Infrastructure<G>>,
) -> Result<HttpResponse, Error> {
    let user_agent_id = r
        .headers()
        .get(header::USER_AGENT)
        .and_then(|hv| hv.to_str().ok())
        .map(UserAgent::new)
        .and_then(UserAgent::into_id);

    let authenticate = Authenticate {
        ip_address: r.peer_addr().map(|addr| addr.ip()),
        referrer: query.referrer,
        user_agent_id,
        arena_id_session_id: query.arena_id.zip(query.session_id),
        invitation_id: query.invitation_id,
    };

    match srv.send(authenticate).await {
        Ok(result) => match result {
            Ok(player_id) => ws::start(
                WebSocket::<Request<G::Command>, Update<G::ClientUpdate>, ()>::new(
                    srv.recipient(),
                    query.protocol.unwrap_or_default(),
                    RateLimiterProps::new(Duration::from_secs_f32(Ticks::PERIOD_SECS * 0.9), 5),
                    player_id,
                    (),
                ),
                &r,
                stream,
            ),
            Err(e) => Ok(HttpResponse::TooManyRequests().body(e.to_string())),
        },
        Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
    }
}

pub fn entry_point<G: GameArenaService>() {
    let options = Options::from_args();

    let mut logger = env_logger::builder();
    logger.format_timestamp(None);
    logger.filter_module("server", options.debug_game);
    logger.filter_module("game_server", options.debug_game);
    logger.filter_module("game_server::system", options.debug_watchdog);
    logger.filter_module("core_protocol", options.debug_core);
    logger.filter_module("server_util::web_socket", options.debug_sockets);
    logger.filter_module("actix_web", options.debug_http);
    logger.filter_module("actix_server", options.debug_http);
    logger.filter_module("server_util::linode", options.debug_watchdog);
    logger.filter_module("server_util::ssl", options.debug_watchdog);
    logger.init();

    let _ = actix_web::rt::System::new().block_on(async move {
        let cloud = options
            .linode_personal_access_token
            .map(|t| Box::new(Linode::new(&t)) as Box<dyn Cloud>);

        let system = cloud
            .zip(options.domain.clone())
            .map(|(cloud, domain)| SystemRepo::<G>::new(cloud, domain));

        let region_id = if let Some(region_id) = options.region_id {
            Some(region_id)
        } else {
            let ip_address = if let Some(ip_address) = options.ip_address {
                Some(ip_address)
            } else {
                SystemRepo::<G>::get_own_public_ip().await
            };

            ip_address.and_then(|ip| SystemRepo::<G>::ip_to_region_id(ip))
        };

        let (early_restart_send, mut early_restart_recv) = tokio::sync::mpsc::channel::<()>(1);

        let srv = Infrastructure::<G>::start(
            Infrastructure::new(
                ServerId::new(options.server_id),
                Some(&REDIRECT_TO_SERVER_ID),
                early_restart_send,
                system,
                game_static_files_hash(),
                region_id,
                options.database_read_only,
                options.min_players,
                options.chat_log,
                options.trace_log,
            )
            .await,
        );
        let domain = Arc::new(options.domain.clone());

        let mut ssl = options
            .certificate_path
            .as_ref()
            .zip(options.private_key_path.as_ref())
            .map(|(certificate_file, private_key_file)| {
                Ssl::new(certificate_file, private_key_file).unwrap()
            });

        let use_ssl = ssl.is_some();

        loop {
            let iter_srv = srv.to_owned();
            let domain_clone = Arc::clone(&domain);

            // If ssl exists, safe to assume whatever certificates exist are now installed.
            if let Some(ssl) = ssl.as_mut() {
                ssl.set_renewed();
            }
            let immut_ssl = &ssl;

            let mut server = HttpServer::new(move || {
                // Rust let's you get away with cloning one closure deep, not all the way to a nested closure.
                let srv_clone = iter_srv.to_owned();
                let srv_clone_admin = iter_srv.to_owned();
                let srv_clone_status = iter_srv.to_owned();
                let srv_clone_system = iter_srv.to_owned();
                let domain_clone = Arc::clone(&domain_clone);

                #[cfg(not(debug_assertions))]
                let domain_clone_cors = domain_clone.as_ref().as_ref().map(|d| {
                    [
                        format!("://{}", d),
                        format!(".{}", d),
                        String::from("http://localhost:8000"),
                    ]
                });

                App::new()
                    // Compile times are O(3^n) on middlewares, so consolidate tasks into as few
                    // middlewares as possible.
                    .wrap_fn(move |mut req, srv| {
                        // Redirect HTTP to HTTPS.
                        if use_ssl && req.connection_info().scheme() != "https" {
                            let url =
                                format!("https://{}{}", req.connection_info().host(), req.uri());
                            let response = req.into_response(
                                HttpResponse::MovedPermanently()
                                    .insert_header((LOCATION, url))
                                    .finish(),
                            );
                            return Box::pin(fut::ready(Ok(response)))
                                as Pin<
                                    Box<
                                        dyn std::future::Future<
                                            Output = Result<ServiceResponse, actix_web::Error>,
                                        >,
                                    >,
                                >;
                        }

                        // Don't redirect index so the url remains intact.
                        // Don't redirect admin, so the server remains controllable.
                        let dont_redirect = if let Some(before_hash) = req.path().split('#').next()
                        {
                            before_hash.starts_with("/admin")
                                || before_hash.starts_with("/status")
                                || before_hash.is_empty()
                                || before_hash == "/"
                        } else {
                            true
                        };
                        if !dont_redirect {
                            if let Some((domain, server_id)) = domain_clone
                                .as_ref()
                                .as_ref()
                                .zip(ServerId::new(REDIRECT_TO_SERVER_ID.load(Ordering::Relaxed)))
                            {
                                let redirect = format!(
                                    "{}://{}.{}{}",
                                    req.uri().scheme_str().unwrap_or("https"),
                                    server_id.0.get(),
                                    domain,
                                    req.path()
                                );
                                let response = req.into_response(
                                    HttpResponse::TemporaryRedirect()
                                        .insert_header((LOCATION, redirect))
                                        .finish(),
                                );
                                return Box::pin(fut::ready(Ok(response)))
                                    as Pin<
                                        Box<
                                            dyn std::future::Future<
                                                Output = Result<ServiceResponse, actix_web::Error>,
                                            >,
                                        >,
                                    >;
                            }
                        }

                        // Some hard-coded redirections.
                        if let Some(accepted) =
                            req.headers().get(ACCEPT).and_then(|v| v.to_str().ok())
                        {
                            if accepted.contains("image/webp") {
                                if let Some(redirect) = match req.path() {
                                    "/sprites_css.png" => Some("/sprites_css.webp"),
                                    "/sprites_webgl.png" => Some("/sprites_webgl.webp"),
                                    "/sand.png" => Some("/sand.webp"),
                                    "/grass.png" => Some("/grass.webp"),
                                    "/snow.png" => Some("/snow.webp"),
                                    _ => None,
                                } {
                                    let mut parts = req.uri().clone().into_parts();
                                    parts.path_and_query =
                                        Some(PathAndQuery::from_static(redirect));
                                    if let Ok(uri) = Uri::from_parts(parts) {
                                        req.head_mut().uri = uri.clone();
                                        req.match_info_mut().set(Url::new(uri));
                                    }
                                }
                            }
                        }

                        // Do the request.
                        Box::pin(srv.call(req).map_ok(|mut res| {
                            let content_length = match res.response().body().size() {
                                BodySize::Sized(n) => n as u32,
                                _ => 0,
                            }
                            .max(1000);
                            if let Some(ip) = res.request().peer_addr().map(|a| a.ip()) {
                                let should_rate_limit = {
                                    HTTP_RATE_LIMITER
                                        .lock()
                                        .unwrap()
                                        .should_limit_rate_with_usage(ip, content_length)
                                };

                                if should_rate_limit {
                                    warn!("Bandwidth limiting {}", ip);

                                    let (request, mut response) = res.into_parts();

                                    // Too many requests status.
                                    *response.status_mut() = StatusCode::from_u16(429).unwrap();

                                    // I changed my mind, I'm not actually going to send you all this data...
                                    response = response.drop_body().map_into_boxed_body();

                                    res = ServiceResponse::new(request, response)
                                }
                            }

                            // Add some universal default headers.
                            for (key, value) in [(CACHE_CONTROL, "no-cache")] {
                                if !res.headers().contains_key(key.clone()) {
                                    res.headers_mut()
                                        .insert(key, HeaderValue::from_static(value));
                                }
                            }
                            res
                        }))
                            as Pin<
                                Box<
                                    dyn std::future::Future<
                                        Output = Result<ServiceResponse, actix_web::Error>,
                                    >,
                                >,
                            >
                    })
                    .wrap(
                        Cors::default()
                            .allow_any_header()
                            .allowed_methods([
                                Method::GET,
                                Method::HEAD,
                                Method::POST,
                                Method::OPTIONS,
                            ])
                            .allowed_origin_fn(move |origin, _head| {
                                #[cfg(debug_assertions)]
                                {
                                    let _ = origin;
                                    true
                                }

                                #[cfg(not(debug_assertions))]
                                if let Some(domains) = domain_clone_cors.as_ref() {
                                    domains.iter().any(|domain| {
                                        origin.as_bytes().ends_with(domain.as_bytes())
                                    })
                                } else {
                                    true
                                }
                            }),
                    )
                    .service(resource("/ws/").route(web::get().to(
                        move |r: HttpRequest,
                              stream: web::Payload,
                              query: Query<WebSocketQuery>| {
                            let query = query.into_inner();
                            ws_index(r, stream, query, srv_clone.to_owned())
                        },
                    )))
                    .service(resource("/admin/").route(post().to(
                        move |request: Json<ParameterizedAdminRequest>| {
                            debug!("received admin request");

                            let srv_clone_admin = srv_clone_admin.clone();

                            async move {
                                match srv_clone_admin.send(request.0).await {
                                    Ok(result) => match result {
                                        actix_web::Result::Ok(update) => {
                                            let response = serde_json::to_vec(&update).unwrap();
                                            HttpResponse::Ok()
                                                .content_type("application/json")
                                                .body(response)
                                        }
                                        Err(e) => HttpResponse::BadRequest().body(String::from(e)),
                                    },
                                    Err(e) => {
                                        HttpResponse::InternalServerError().body(e.to_string())
                                    }
                                }
                            }
                        },
                    )))
                    .service(resource("/status/").route(get().to(move || {
                        let srv = srv_clone_status.to_owned();
                        debug!("received status request");

                        async move {
                            match srv.send(StatusRequest).await {
                                Ok(status_response) => {
                                    let response = serde_json::to_vec(&status_response).unwrap();
                                    HttpResponse::Ok()
                                        .content_type("application/json")
                                        .body(response)
                                }
                                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                            }
                        }
                    })))
                    .service(resource("/system/").route(get().to(
                        move |r: HttpRequest, query: Query<SystemQuery>| {
                            let srv = srv_clone_system.to_owned();
                            debug!("received system request");

                            let ip = r.peer_addr().map(|addr| addr.ip());

                            async move {
                                match srv
                                    .send(SystemRequest {
                                        ip,
                                        server_id: query.server_id,
                                        region_id: query.region_id,
                                        invitation_id: query.invitation_id,
                                    })
                                    .await
                                {
                                    Ok(system_response) => {
                                        let response =
                                            serde_json::to_vec(&system_response).unwrap();
                                        HttpResponse::Ok()
                                            .content_type("application/json")
                                            .body(response)
                                    }
                                    Err(e) => {
                                        HttpResponse::InternalServerError().body(e.to_string())
                                    }
                                }
                            }
                        },
                    )))
                    .configure(static_files())
            })
            .on_connect(on_connect_enable_nodelay);

            if let Some(ssl) = immut_ssl {
                server = server
                    .bind_rustls("0.0.0.0:443", ssl.rustls_config())
                    .expect("could not listen (https)");
                server = server.bind("0.0.0.0:80").expect("could not listen (http)");
            } else {
                server = server
                    .bind("0.0.0.0:8000")
                    .expect("could not listen (http)");
            }

            let running_server = server
                .keep_alive(KEEP_ALIVE)
                .shutdown_timeout(SHUTDOWN_TIMEOUT)
                .max_connections(max_connections_per_worker())
                .backlog(BACKLOG)
                .run();

            if run_until_ssl_renewal(running_server, immut_ssl, &mut early_restart_recv)
                .await
                .is_err()
            {
                break;
            }
        }
    });
}
