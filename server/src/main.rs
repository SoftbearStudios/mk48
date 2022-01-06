// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(drain_filter)]
#![feature(new_uninit)]
#![feature(get_mut_unchecked)]
#![feature(async_closure)]
#![feature(hash_drain_filter)]

//! The game server has authority over all game logic. Clients are served the client, which connects
//! via websocket.

use crate::protocol::Authenticate;
use actix::prelude::*;
use actix_cors::Cors;
use actix_web::dev::{Service, ServiceResponse, Url};
use actix_web::http::header::{HeaderValue, ACCEPT, CACHE_CONTROL, LOCATION};
use actix_web::http::uri::PathAndQuery;
use actix_web::http::{Method, Uri};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use common::entity::EntityType;
use common::protocol::{Command, Update};
use core_protocol::dto::InvitationDto;
use core_protocol::id::*;
use core_protocol::web_socket::WebSocketFormat;
use core_server::app::core_services;

use futures::TryFutureExt;
use log::LevelFilter;
use serde::Deserialize;
use server_util::app::{include_dir, static_files};
use server_util::cloud::Cloud;
use server_util::linode::Linode;
use server_util::ssl::{run_until_ssl_renewal, Ssl};
use server_util::tcp::{
    max_connections_per_worker, on_connect_enable_nodelay, BACKLOG, KEEP_ALIVE, SHUTDOWN_TIMEOUT,
};
use server_util::watchdog;
use server_util::web_socket::WebSocket;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use structopt::StructOpt;

mod arena;
mod bot;
mod collision;
mod complete_ref;
mod contact_ref;
mod entities;
mod entity;
mod entity_extension;
mod noise;
mod player;
mod protocol;
mod server;
mod ups_monitor;
mod world;
mod world_inbound;
mod world_mutation;
mod world_outbound;
mod world_physics;
mod world_physics_radius;
mod world_spawn;

/// Server options, to be specified as arguments.
#[derive(Debug, StructOpt)]
struct Options {
    /// Minimum player count (to be achieved by adding bots)
    #[structopt(short = "p", long, default_value = "30")]
    min_players: usize,
    /// Log incoming HTTP requests
    #[cfg_attr(debug_assertions, structopt(long, default_value = "warn"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    debug_http: LevelFilter,
    /// Log game diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "info"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    debug_game: LevelFilter,
    /// Log core diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "info"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    debug_core: LevelFilter,
    /// Log socket diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "warn"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    debug_sockets: LevelFilter,
    /// Log watchdog diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "info"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "warn"))]
    debug_watchdog: LevelFilter,
    /// Log chats
    #[structopt(long)]
    chat_log: Option<String>,
    #[structopt(long)]
    linode_personal_access_token: Option<String>,
    // Don't write to the database.
    #[structopt(long)]
    database_read_only: bool,
    // Server id.
    #[structopt(long, default_value = "0")]
    server_id: u8,
    /// Domain (without server id prepended).
    #[allow(dead_code)]
    #[structopt(long)]
    domain: Option<String>,
    // Certificate chain path
    #[structopt(long)]
    certificate_path: Option<String>,
    // Private key path
    #[structopt(long)]
    private_key_path: Option<String>,
}

#[derive(Deserialize)]
struct WebSocketFormatQuery {
    format: Option<WebSocketFormat>,
}

/// ws_index routes incoming HTTP requests to WebSocket connections.
async fn ws_index(
    r: HttpRequest,
    stream: web::Payload,
    session_id: SessionId,
    format: WebSocketFormat,
    srv: Addr<server::Server>,
) -> Result<HttpResponse, Error> {
    match srv.send(Authenticate { session_id }).await {
        Ok(response) => match response {
            Some((player_id, invitation)) => ws::start(
                WebSocket::<Command, Update, (SessionId, PlayerId, Option<InvitationDto>)>::new(
                    srv.recipient(),
                    format,
                    (session_id, player_id, invitation),
                ),
                &r,
                stream,
            ),
            None => Ok(HttpResponse::Unauthorized().body("invalid session id")),
        },
        Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
    }
}

/// 0 is no redirect.
static REDIRECT_TO_SERVER_ID: AtomicU8 = AtomicU8::new(0);

fn main() {
    // SAFETY: As per spec, only called once (before .data()) is called.
    unsafe {
        EntityType::init();
        noise::init();

        for typ in EntityType::iter() {
            rustrict::add_word(typ.as_str(), rustrict::Type::SAFE);
        }
    }

    let options = Options::from_args();

    let mut logger = env_logger::builder();
    logger.format_timestamp(None);
    logger.filter_module(module_path!(), options.debug_game);
    logger.filter_module("core_server", options.debug_core);
    logger.filter_module("core_protocol", options.debug_core);
    logger.filter_module("server_util::web_socket", options.debug_sockets);
    logger.filter_module("actix_web", options.debug_http);
    logger.filter_module("actix_server", options.debug_http);
    logger.filter_module("server_util::watchdog", options.debug_watchdog);
    logger.filter_module("server_util::linode", options.debug_watchdog);
    logger.filter_module("server_util::ssl", options.debug_watchdog);
    logger.init();

    let _ = actix_web::rt::System::new().block_on(async move {
        let cloud = options
            .linode_personal_access_token
            .map(|t| Box::new(Linode::new(&t)) as Box<dyn Cloud>);

        let core = core_server::core::Core::start(
            core_server::core::Core::new(
                options.chat_log,
                options.database_read_only,
                Some(&REDIRECT_TO_SERVER_ID),
            )
            .await,
        );
        let srv = server::Server::start(server::Server::new(
            ServerId::new(options.server_id),
            options.min_players,
            core.to_owned(),
        ));
        let domain = Arc::new(options.domain.clone());
        if let Some((cloud, domain)) = cloud.zip(options.domain) {
            // Safety: Only happens once.
            unsafe {
                watchdog::Watchdog::start(watchdog::Watchdog::new(cloud, domain));
            }
        }

        let mut ssl = options
            .certificate_path
            .as_ref()
            .zip(options.private_key_path.as_ref())
            .map(|(certificate_file, private_key_file)| {
                Ssl::new(certificate_file, private_key_file).unwrap()
            });

        let use_ssl = ssl.is_some();

        loop {
            let iter_core = core.to_owned();
            let iter_srv = srv.to_owned();
            let domain_clone = Arc::clone(&domain);

            // If ssl exists, safe to assume whatever certificates exist are now installed.
            if let Some(ssl) = ssl.as_mut() {
                ssl.set_renewed();
            }
            let immut_ssl = &ssl;

            let mut server = HttpServer::new(move || {
                // Rust let's you get away with cloning one closure deep, not all the way to a nested closure.
                let core_clone = iter_core.to_owned();
                let srv_clone = iter_srv.to_owned();
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
                    .service(web::resource("/ws/{session_id}/").route(web::get().to(
                        move |r: HttpRequest,
                              stream: web::Payload,
                              path: web::Path<SessionId>,
                              query: web::Query<WebSocketFormatQuery>| {
                            ws_index(
                                r,
                                stream,
                                path.into_inner(),
                                query.into_inner().format.unwrap_or_default(),
                                srv_clone.to_owned(),
                            )
                        },
                    )))
                    .configure(core_services(core_clone))
                    .configure(static_files(&include_dir!("../js/public")))
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

            if run_until_ssl_renewal(running_server, immut_ssl)
                .await
                .is_err()
            {
                break;
            }
        }
    });
}
