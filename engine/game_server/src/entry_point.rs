// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The game server has authority over all game logic. Clients are served the client, which connects
//! via web_socket.

use crate::admin::ParameterizedAdminRequest;
use crate::client::Authenticate;
use crate::game_service::GameArenaService;
use crate::infrastructure::Infrastructure;
use crate::static_files::{static_handler, static_hash};
use crate::status::StatusRequest;
use crate::system::{SystemRepo, SystemRequest};
use actix::Actor;
use axum::body::HttpBody;
use axum::extract::ws::{CloseCode, CloseFrame, Message};
use axum::extract::{ConnectInfo, Query, TypedHeader, WebSocketUpgrade};
use axum::http::uri::Scheme;
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Redirect};
use axum::routing::get;
use axum::Router;
use bincode::{self, Options as _};
use core_protocol::id::*;
use core_protocol::rpc::{Request, SystemQuery, Update, WebSocketQuery};
use core_protocol::web_socket::WebSocketProtocol;
use core_protocol::{get_unix_time_now, UnixTime};
use futures::pin_mut;
use futures::SinkExt;
use log::{debug, error, warn, LevelFilter};
use rust_embed::RustEmbed;
use server_util::cloud::Cloud;
use server_util::ip_rate_limiter::IpRateLimiter;
use server_util::linode::Linode;
use server_util::observer::{ObserverMessage, ObserverUpdate};
use server_util::rate_limiter::{RateLimiterProps, RateLimiterState};
use server_util::user_agent::UserAgent;
use std::convert::TryInto;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use structopt::StructOpt;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

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
    /// Certificate chain path.
    #[structopt(long)]
    pub certificate_path: Option<String>,
    /// Private key path.
    #[structopt(long)]
    pub private_key_path: Option<String>,
    /// HTTP request bandwidth limiting (in bytes per second).
    #[structopt(long, default_value = "500000")]
    pub http_bandwidth_limit: u32,
    /// HTTP request rate limiting burst (in bytes).
    #[structopt(long, default_value = "10000000")]
    pub http_bandwidth_burst: u32,
    /// Client authenticate rate limiting period (in seconds).
    #[structopt(long, default_value = "30")]
    pub client_authenticate_rate_limit: u64,
    /// Client authenticate rate limiting burst.
    #[structopt(long, default_value = "16")]
    pub client_authenticate_burst: u32,
}

/// 0 is no redirect.
static REDIRECT_TO_SERVER_ID: AtomicU8 = AtomicU8::new(0);

lazy_static::lazy_static! {
    // Will be overwritten first thing.
    static ref HTTP_RATE_LIMITER: Mutex<IpRateLimiter> = Mutex::new(IpRateLimiter::new_bandwidth_limiter(1, 0));
}

#[derive(RustEmbed)]
#[folder = "../../client_static/"]
struct GameClient;

#[derive(RustEmbed)]
#[folder = "../js/public/"]
#[prefix = "admin/"]
struct AdminClient;

pub fn entry_point<G: GameArenaService>() {
    let options = Options::from_args();

    let mut logger = env_logger::builder();
    logger.format_timestamp(None);
    logger.filter_module("server", options.debug_game);
    logger.filter_module("game_server", options.debug_game);
    logger.filter_module("game_server::system", options.debug_watchdog);
    logger.filter_module("core_protocol", options.debug_core);
    logger.filter_module("server_util::web_socket", options.debug_sockets);
    logger.filter_module("server_util::linode", options.debug_watchdog);
    logger.filter_module("server_util::ssl", options.debug_watchdog);
    logger.init();

    *HTTP_RATE_LIMITER.lock().unwrap() = IpRateLimiter::new_bandwidth_limiter(
        options.http_bandwidth_limit,
        options.http_bandwidth_burst,
    );

    let _ = actix::System::new().block_on(async move {
        let cloud = options
            .linode_personal_access_token
            .map(|t| Box::new(Linode::new(&t)) as Box<dyn Cloud>);

        let system = cloud
            .zip(options.domain.clone())
            .map(|(cloud, domain)| SystemRepo::<G>::new(cloud, domain, &REDIRECT_TO_SERVER_ID));

        let server_id = ServerId::new(options.server_id);
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

        let srv = Infrastructure::<G>::start(
            Infrastructure::new(
                server_id,
                system,
                static_hash::<GameClient>(),
                region_id,
                options.database_read_only,
                options.min_players,
                options.chat_log,
                options.trace_log,
                RateLimiterProps::new(
                    Duration::from_secs(options.client_authenticate_rate_limit),
                    options.client_authenticate_burst,
                ),
            )
            .await,
        );
        let domain = &*Box::leak(Box::new(options.domain.clone()));

        #[cfg(not(debug_assertions))]
        let certificate_paths = options
            .certificate_path
            .as_ref()
            .zip(options.private_key_path.as_ref());

        let ws_srv = srv.to_owned();
        let admin_srv = srv.to_owned();
        let status_srv = srv.to_owned();
        let system_srv = srv.to_owned();

        #[cfg(not(debug_assertions))]
        let domain_clone_cors = domain.as_ref().map(|d| {
            [
                format!("://{}", d),
                format!(".{}", d),
                String::from("http://localhost:8000"),
                String::from("https://localhost:8001"),
                String::from("http://localhost:80"),
                String::from("https://localhost:443"),
            ]
        });

        let app = Router::new()
            .fallback(get(static_handler::<GameClient>))
            .route("/ws/", axum::routing::get(async move |upgrade: WebSocketUpgrade, addr: Option<ConnectInfo<SocketAddr>>, user_agent: Option<TypedHeader<axum::headers::UserAgent>>, query: Query<WebSocketQuery>| {
                let user_agent_id = user_agent
                    .map(|h| UserAgent::new(h.as_str()))
                    .and_then(UserAgent::into_id);

                let authenticate = Authenticate {
                    ip_address: addr.map(|addr| addr.0.ip()),
                    referrer: query.referrer,
                    user_agent_id,
                    arena_id_session_id: query.arena_id.zip(query.session_id),
                    invitation_id: query.invitation_id,
                };

                const MAX_MESSAGE_SIZE: usize = 32768;
                const TIMER_SECONDS: u64 = 10;
                const TIMER_DURATION: Duration = Duration::from_secs(TIMER_SECONDS);
                const WEBSOCK_HARD_TIMEOUT: Duration = Duration::from_secs(TIMER_SECONDS * 2);

                match ws_srv.send(authenticate).await {
                    Ok(result) => match result {
                        Ok(player_id) => Ok(upgrade
                            .max_frame_size(MAX_MESSAGE_SIZE)
                            .max_message_size(MAX_MESSAGE_SIZE)
                            .max_send_queue(32)
                            .on_upgrade(async move |mut web_socket| {
                            let mut protocol = query.protocol.unwrap_or_default();

                            let (server_sender, mut server_receiver) = tokio::sync::mpsc::unbounded_channel::<ObserverUpdate<Update<G::ClientUpdate>>>();

                            let _ = ws_srv.do_send(ObserverMessage::<Request<G::Command>, Update<G::ClientUpdate>>::Register {
                                player_id,
                                observer: server_sender.clone(),
                            });

                            let keep_alive = tokio::time::sleep(TIMER_DURATION);
                            let mut last_activity = Instant::now();
                            let mut rate_limiter = RateLimiterState::default();
                            const RATE: RateLimiterProps = RateLimiterProps::const_new(Duration::from_millis(80), 5);

                            pin_mut!(keep_alive);

                            // For signaling what type of close frame should be sent, if any.
                            const NORMAL_CLOSURE: Option<CloseCode> = Some(1000);
                            const PROTOCOL_ERROR: Option<CloseCode> = Some(1002);
                            const SILENT_CLOSURE: Option<CloseCode> = None;

                            let closure = loop {
                                tokio::select! {
                                    web_socket_update = web_socket.recv() => {
                                        match web_socket_update {
                                            Some(result) => match result {
                                                Ok(message) => {
                                                    last_activity = Instant::now();
                                                    keep_alive.as_mut().reset((last_activity + TIMER_DURATION).into());

                                                    match message {
                                                        Message::Binary(binary) => {
                                                            if rate_limiter.should_limit_rate_with_now(&RATE, last_activity) {
                                                                continue;
                                                            }

                                                            match bincode::DefaultOptions::new()
                                                                .with_limit(MAX_MESSAGE_SIZE as u64)
                                                                .with_fixint_encoding()
                                                                .allow_trailing_bytes()
                                                                .deserialize(binary.as_ref())
                                                            {
                                                                Ok(request) => {
                                                                    protocol = WebSocketProtocol::Binary;
                                                                    let _ = ws_srv.do_send(ObserverMessage::<Request<G::Command>, Update<G::ClientUpdate>>::Request {
                                                                        player_id,
                                                                        request,
                                                                    });
                                                                }
                                                                Err(err) => {
                                                                    warn!("deserialize binary err ignored {}", err);
                                                                }
                                                            }
                                                        }
                                                        Message::Text(text) => {
                                                            if rate_limiter.should_limit_rate_with_now(&RATE, last_activity) {
                                                                continue;
                                                            }

                                                            let result: Result<Request<G::Command>, serde_json::Error> = serde_json::from_str(&text);
                                                            match result {
                                                                Ok(request) => {
                                                                    protocol = WebSocketProtocol::Json;
                                                                    let _ = ws_srv.do_send(ObserverMessage::<Request<G::Command>, Update<G::ClientUpdate>>::Request {
                                                                        player_id,
                                                                        request,
                                                                    });
                                                                }
                                                                Err(err) => {
                                                                    warn!("parse err ignored {}", err);
                                                                }
                                                            }
                                                        }
                                                        Message::Ping(_) => {
                                                            // Axum spec days that automatic Pong will be sent.
                                                        }
                                                        Message::Pong(pong_data) => {
                                                            if rate_limiter.should_limit_rate_with_now(&RATE, last_activity) {
                                                                continue;
                                                            }

                                                            if let Ok(bytes) = pong_data.try_into() {
                                                                let now = get_unix_time_now();
                                                                let timestamp = UnixTime::from_ne_bytes(bytes);
                                                                let rtt = now.saturating_sub(timestamp);
                                                                if rtt < u16::MAX as UnixTime {
                                                                    let _ = ws_srv.do_send(ObserverMessage::<Request<G::Command>, Update<G::ClientUpdate>>::RoundTripTime {
                                                                        player_id,
                                                                        rtt: rtt as u16,
                                                                    });
                                                                }
                                                            } else {
                                                                debug!("received invalid pong data");
                                                            }
                                                        },
                                                        Message::Close(_) => {
                                                            debug!("recieved close from client");
                                                            // tungstenite will echo close frame if necessary.
                                                            break SILENT_CLOSURE;
                                                        },
                                                    }
                                                }
                                                Err(error) => {
                                                    debug!("web socket error: {:?}", error);
                                                    break PROTOCOL_ERROR;
                                                }
                                            }
                                            None => {
                                                // web socket closed already.
                                                break SILENT_CLOSURE;
                                            }
                                        }
                                    },
                                    maybe_observer_update = server_receiver.recv() => {
                                        let observer_update = match maybe_observer_update {
                                            Some(observer_update) => observer_update,
                                            None => {
                                                // infrastructure wants websocket closed.
                                                break NORMAL_CLOSURE
                                            }
                                        };
                                        match observer_update {
                                            ObserverUpdate::Send{message} => {
                                                let web_socket_message = match protocol {
                                                    WebSocketProtocol::Binary => Message::Binary(bincode::serialize(&message).unwrap()),
                                                    WebSocketProtocol::Json => Message::Text(serde_json::to_string(&message).unwrap()),
                                                };
                                                if web_socket.send(web_socket_message).await.is_err() {
                                                    break NORMAL_CLOSURE;
                                                }
                                            }
                                            ObserverUpdate::Close => {
                                                break NORMAL_CLOSURE;
                                            }
                                        }
                                    },
                                    _ = keep_alive.as_mut() => {
                                        if last_activity.elapsed() < WEBSOCK_HARD_TIMEOUT {
                                            if web_socket.send(Message::Ping(get_unix_time_now().to_ne_bytes().into())).await.is_err() {
                                                break NORMAL_CLOSURE;
                                            }
                                            keep_alive.as_mut().reset((Instant::now() + TIMER_DURATION).into());
                                        } else {
                                            debug!("closing unresponsive");
                                            break PROTOCOL_ERROR;
                                        }
                                    }
                                }
                            };

                            let _ = ws_srv.do_send(ObserverMessage::<Request<G::Command>, Update<G::ClientUpdate>>::Unregister {
                                player_id,
                                observer: server_sender,
                            });

                            if let Some(code) = closure {
                                let _ = web_socket.send(Message::Close(Some(CloseFrame{code, reason: "".into()}))).await;
                            } else {
                                let _ = web_socket.flush().await;
                            }
                        })),
                        Err(_) => Err(StatusCode::TOO_MANY_REQUESTS.into_response()),
                    },
                    Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
                }
            }))
            .route("/system/", axum::routing::get(move |addr: Option<ConnectInfo<SocketAddr>>, query: Query<SystemQuery>| {
                let srv = system_srv.to_owned();
                debug!("received system request");

                let ip = addr.map(|addr| addr.0.ip());

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
                            Ok(axum::Json(system_response))
                        }
                        Err(e) => {
                            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
                        }
                    }
                }
            }))
            .layer(ServiceBuilder::new()
                .layer(CorsLayer::new()
                    .allow_origin(tower_http::cors::Origin::predicate(move |origin, _parts| {
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
                    }))
                    .allow_headers(tower_http::cors::Any)
                    .allow_methods([axum::http::method::Method::GET, axum::http::method::Method::HEAD, axum::http::method::Method::POST, axum::http::method::Method::OPTIONS]))
                .layer(axum::middleware::from_fn(async move |request: axum::http::Request<_>, next: axum::middleware::Next<_>| {
                    // Don't redirect index so the url remains intact.
                    // Don't redirect admin or status, so the server remains controllable.
                    let dont_redirect = if let Some(before_hash) = request.uri().path().split('#').next()
                    {
                        before_hash.starts_with("/admin")
                            || before_hash.starts_with("/status")
                            || before_hash.is_empty()
                            || before_hash == "/"
                    } else {
                        true
                    };

                    if !dont_redirect {
                        if let Some((domain, server_id)) = domain
                            .as_ref()
                            .zip(ServerId::new(REDIRECT_TO_SERVER_ID.load(Ordering::Relaxed)))
                        {
                            let scheme = request.uri().scheme().cloned().unwrap_or(Scheme::HTTPS);
                            if let Ok(uri) = Uri::builder()
                                .scheme(scheme)
                                .path_and_query(format!("{}.{}{}", server_id.0.get(), domain, request.uri().path()))
                                .build(){
                                return Err(Redirect::temporary(uri));
                            }
                        }
                    }

                    let ip = request.extensions().get::<ConnectInfo<SocketAddr>>().map(|ci| ci.0.ip());

                    let mut response = next.run(request).await;

                    let content_length = response
                        .headers()
                        .get(axum::http::header::CONTENT_LENGTH)
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| u32::from_str(s).ok())
                        .unwrap_or(response.body().size_hint().lower() as u32)
                        .max(500);

                    if let Some(ip) = ip {
                        let should_rate_limit = {
                            HTTP_RATE_LIMITER
                                .lock()
                                .unwrap()
                                .should_limit_rate_with_usage(ip, content_length)
                        };

                        if should_rate_limit {
                            warn!("Bandwidth limiting {}", ip);

                            *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;

                            // I changed my mind, I'm not actually going to send you all this data...
                            response = response.map(|_| {
                                axum::body::boxed(axum::body::Empty::new())
                            });
                        }
                    }

                    Ok(response)
                }))
            )
            .route("/status/", axum::routing::get(move || {
                let srv = status_srv.to_owned();
                debug!("received status request");

                async move {
                    match srv.send(StatusRequest).await {
                        Ok(status_response) => {
                            Ok(axum::Json(status_response))
                        }
                        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
                    }
                }
            }))
            .route("/admin/*path", axum::routing::get(static_handler::<AdminClient>).post(
                move |request: axum::extract::Json<ParameterizedAdminRequest>| {
                    let srv_clone_admin = admin_srv.clone();

                    async move {
                        match srv_clone_admin.send(request.0).await {
                            Ok(result) => match result {
                                Ok(update) => {
                                    Ok(axum::Json(update))
                                }
                                Err(e) => Err((axum::http::status::StatusCode::BAD_REQUEST, String::from(e)).into_response()),
                            },
                            Err(e) => {
                                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
                            }
                        }
                    }
                }
            ));

        let addr_incoming_config = axum_server::AddrIncomingConfig::new()
            .tcp_keepalive(Some(Duration::from_secs(32)))
            .tcp_nodelay(true)
            .tcp_sleep_on_accept_errors(true)
            .build();

        let http_config = axum_server::HttpConfig::new()
            .http2_max_concurrent_streams(Some(8))
            .http2_keep_alive_interval(Some(Duration::from_secs(4)))
            .http2_keep_alive_timeout(Duration::from_secs(10))
            .build();

        let ports = if nix::unistd::Uid::effective().is_root() {
            (80, 443)
        } else {
            (8000, 8001)
        };

        #[cfg(not(debug_assertions))]
        let http_app = Router::new()
            .fallback(get(async move |uri: Uri, host: TypedHeader<axum::headers::Host>| {
                let mut parts = uri.into_parts();
                parts.scheme = Some(Scheme::HTTPS);
                let authority_str = format!("{}:{}", host.0.hostname(), ports.1);
                let authority = axum::http::uri::Authority::from_str(&authority_str)
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
                parts.authority = Some(authority);
                Uri::from_parts(parts)
                    .map(|uri| Redirect::permanent(uri))
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
            }));

        #[cfg(debug_assertions)]
        let http_app = app;

        let http_server = axum_server::bind(SocketAddr::from(([0, 0, 0, 0], ports.0)))
            .addr_incoming_config(addr_incoming_config.clone())
            .http_config(http_config.clone())
            .serve(http_app.into_make_service_with_connect_info::<SocketAddr, _>());

        #[cfg(debug_assertions)]
        error!("http server stopped: {:?}", http_server.await);

        #[cfg(not(debug_assertions))]
        let rustls_config = if let Some((certificate_path, private_key_path)) = certificate_paths {
            let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                certificate_path,
                private_key_path,
            ).await.unwrap();

            let renewal_rustls_config = rustls_config.clone();
            let certificate_path = certificate_path.to_owned();
            let private_key_path = private_key_path.to_owned();

            tokio::spawn(async move {
                let mut old_expiry = server_util::ssl::certificate_expiry(&certificate_path).unwrap();

                const CHECK_PERIOD: Duration = Duration::from_secs(24 * 3600);
                let mut governor = tokio::time::interval(CHECK_PERIOD);

                loop {
                    governor.tick().await;

                    match server_util::ssl::certificate_expiry(&certificate_path) {
                        Ok(new_expiry) => {
                            if new_expiry > old_expiry {
                                warn!("renewing SSL certificate...");
                                if let Err(e) = renewal_rustls_config.reload_from_pem_file(&certificate_path, &private_key_path).await {
                                    error!("failed to renew SSL certificate: {}", e);
                                } else {
                                    old_expiry = new_expiry;
                                }
                            } else {
                                log::info!("SSL certificate not in need of renewal.");
                            }
                        }
                        Err(e) => error!("failed to get SSL certificate expiry: {}", e)
                    }
                }
            });

            rustls_config
        } else {
            warn!("Using self-signed certificate in place of trusted certificate.");
            axum_server::tls_rustls::RustlsConfig::from_pem(
                include_bytes!("certificate.pem").as_slice().into(),
                include_bytes!("private_key.pem").as_slice().into(),
            ).await.unwrap()
        };

        #[cfg(not(debug_assertions))]
        let https_server = axum_server::bind_rustls(SocketAddr::from(([0, 0, 0, 0], ports.1)), rustls_config)
            .addr_incoming_config(addr_incoming_config.clone())
            .http_config(http_config)
            .serve(app.into_make_service_with_connect_info::<SocketAddr, _>());

        #[cfg(not(debug_assertions))]
        tokio::select! {
            result = http_server => {
                error!("http server stopped: {:?}", result);
            }
            result = https_server => {
                error!("https server stopped: {:?}", result);
            }
        }
    });
}
