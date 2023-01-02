// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The game server has authority over all game logic. Clients are served the client, which connects
//! via web_socket.

use crate::admin::ParameterizedAdminRequest;
use crate::client::{Authenticate, Oauth2Code};
use crate::discord::{DiscordBotRepo, DiscordOauth2Repo};
use crate::game_service::GameArenaService;
use crate::infrastructure::Infrastructure;
use crate::leaderboard::LeaderboardRequest;
use crate::options::Options;
use crate::static_files::{static_size_and_hash, StaticFilesHandler};
use crate::status::StatusRequest;
use crate::system::{SystemRepo, SystemRequest};
use actix::Actor;
use axum::body::{boxed, Empty, Full, HttpBody};
use axum::extract::ws::{CloseCode, CloseFrame, Message};
use axum::extract::{ConnectInfo, Query, TypedHeader, WebSocketUpgrade};
use axum::headers::HeaderName;
use axum::http::header::CACHE_CONTROL;
use axum::http::uri::{Authority, Scheme};
use axum::http::{HeaderValue, Method, Response, StatusCode, Uri};
use axum::response::{IntoResponse, Redirect};
use axum::routing::get;
use axum::{Json, Router};
use bincode::{self, Options as _};
use core_protocol::id::*;
use core_protocol::rpc::{Request, SystemQuery, Update, WebSocketQuery};
use core_protocol::web_socket::WebSocketProtocol;
use core_protocol::{get_unix_time_now, UnixTime};
use futures::pin_mut;
use futures::SinkExt;
use log::{debug, error, info, warn};
use minicdn::release_include_mini_cdn;
use minicdn::MiniCdn;
use server_util::cloud::Cloud;
use server_util::http::limit_content_length;
use server_util::ip_rate_limiter::IpRateLimiter;
use server_util::linode::Linode;
use server_util::observer::{ObserverMessage, ObserverUpdate};
use server_util::os::set_open_file_limit;
use server_util::rate_limiter::{RateLimiterProps, RateLimiterState};
use server_util::user_agent::UserAgent;
use std::convert::TryInto;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use structopt::StructOpt;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

/// 0 is no redirect.
static REDIRECT_TO_SERVER_ID: AtomicU8 = AtomicU8::new(0);

/// Whether JSON is allowed for WebSockets. This may be disabled as a defense mechanism against
/// denial of service or unwanted bots.
static ALLOW_WEB_SOCKET_JSON: AtomicBool = AtomicBool::new(true);

lazy_static::lazy_static! {
    // Will be overwritten first thing.
    static ref HTTP_RATE_LIMITER: Mutex<IpRateLimiter> = Mutex::new(IpRateLimiter::new_bandwidth_limiter(1, 0));
}

pub fn entry_point<G: GameArenaService>(game_client: MiniCdn, browser_router: bool) {
    let _ = actix::System::new().block_on(async move {
        let options = Options::from_args();

        crate::log::init_logger(&options);

        match set_open_file_limit(16384) {
            Ok(limit) => info!("set open file limit to {}", limit),
            Err(e) => error!("could not set open file limit: {}", e)
        }

        #[allow(unused)]
        let (http_port, https_port) = options.http_and_https_ports();

        let (static_size, static_hash) = static_size_and_hash(&game_client);
        let bandwidth_burst = options.bandwidth_burst(static_size);

        *HTTP_RATE_LIMITER.lock().unwrap() =
            IpRateLimiter::new_bandwidth_limiter(options.http_bandwidth_limit, bandwidth_burst);

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

        let game_client = Arc::new(RwLock::new(game_client));
        let admin_client = Arc::new(RwLock::new(release_include_mini_cdn!("../../js/public")));
        let discord_guild_id = options.discord_guild_id;
        let discord_bot = options.discord_bot_token.and_then(|t| DiscordBotRepo::new(discord_guild_id, &t));
        let discord_client_id = options.discord_client_id;
        let domain = options.domain.map(|domain| &*Box::leak(domain.into_boxed_str()));
        let discord_oauth2 = options.discord_client_secret
            .map(|client_secret| &*Box::leak(Box::new(DiscordOauth2Repo::new(
                discord_client_id,
                client_secret,
                domain
                    .filter(|_| cfg!(not(debug_assertions)))
                    .map(|d| format!("https://{d}"))
                    .unwrap_or_else(|| format!("http://localhost:{http_port}"))
            ))));

        // println!("{:?}", discord_bot.as_ref().unwrap().send_message("", "", None).await);

        let srv = Infrastructure::<G>::start(
            Infrastructure::new(
                server_id,
                system,
                discord_bot,
                discord_oauth2,
                static_hash,
                region_id,
                options.database_read_only,
                options.min_bots,
                options.max_bots,
                options.bot_percent,
                options.chat_log,
                options.trace_log,
                Arc::clone(&game_client),
                &ALLOW_WEB_SOCKET_JSON,
                options.admin_config_file,
                RateLimiterProps::new(
                    Duration::from_secs(options.client_authenticate_rate_limit),
                    options.client_authenticate_burst,
                ),
            )
            .await,
        );

        #[cfg(not(debug_assertions))]
        let certificate_paths = options
            .certificate_path
            .as_ref()
            .zip(options.private_key_path.as_ref());

        let ws_srv = srv.to_owned();
        let admin_srv = srv.to_owned();
        let leaderboard_srv = srv.to_owned();
        let status_srv = srv.to_owned();
        let system_srv = srv.to_owned();

        #[cfg(not(debug_assertions))]
        let domain_clone_cors = domain.as_ref().map(|d| {
            [
                format!("://{}", d),
                format!(".{}", d),
                String::from("http://localhost:8080"),
                String::from("https://localhost:8443"),
                String::from("http://localhost:80"),
                String::from("https://localhost:443"),
                String::from("https://softbear.com"),
                String::from("https://www.softbear.com"),
            ]
        });

        let admin_router = get(StaticFilesHandler{cdn: admin_client, prefix: "/admin", browser_router: false}).post(
            move |request: Json<ParameterizedAdminRequest>| {
                let srv_clone_admin = admin_srv.clone();

                async move {
                    match srv_clone_admin.send(request.0).await {
                        Ok(result) => match result {
                            Ok(update) => {
                                Ok(Json(update))
                            }
                            Err(e) => Err((StatusCode::BAD_REQUEST, String::from(e)).into_response()),
                        },
                        Err(e) => {
                            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
                        }
                    }
                }
            }
        );

        let app = Router::new()
            .fallback_service(get(StaticFilesHandler{cdn: game_client, prefix: "", browser_router}))
            .route("/oauth2/discord", get(async move || {
                discord_oauth2.map(|oauth2| oauth2.redirect().into_response()).unwrap_or_else(|| Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(boxed(Full::from("404 Not Found")))
                    .unwrap())
            }))
            .route("/ws", axum::routing::get(async move |upgrade: WebSocketUpgrade, ConnectInfo(addr): ConnectInfo<SocketAddr>, user_agent: Option<TypedHeader<axum::headers::UserAgent>>, Query(query): Query<WebSocketQuery>| {
                let user_agent_id = user_agent
                    .map(|h| UserAgent::new(h.as_str()))
                    .and_then(UserAgent::into_id);
                let login_type = query.login_type;

                let authenticate = Authenticate {
                    ip_address: addr.ip(),
                    referrer: query.referrer,
                    user_agent_id,
                    arena_id_session_id: query.arena_id.zip(query.session_id),
                    invitation_id: query.invitation_id,
                    oauth2_code: query.login_id.filter(|id| id.len() <= 2048 && login_type == Some(LoginType::Discord)).map(Oauth2Code::Discord),
                };

                const MAX_MESSAGE_SIZE: usize = 32768;
                const TIMER_SECONDS: u64 = 10;
                const TIMER_DURATION: Duration = Duration::from_secs(TIMER_SECONDS);
                const WEBSOCKET_HARD_TIMEOUT: Duration = Duration::from_secs(TIMER_SECONDS * 2);

                let mut protocol = query.protocol.unwrap_or_default();
                match ws_srv.send(authenticate).await {
                    Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
                    Ok(result) => match result {
                        // Currently, if authentication fails, it was due to rate limit.
                        Err(_) => Err(StatusCode::TOO_MANY_REQUESTS.into_response()),
                        Ok(player_id) => Ok(upgrade
                            .max_frame_size(MAX_MESSAGE_SIZE)
                            .max_message_size(MAX_MESSAGE_SIZE)
                            .max_send_queue(32)
                            .on_upgrade(async move |mut web_socket| {
                            let (server_sender, mut server_receiver) = tokio::sync::mpsc::unbounded_channel::<ObserverUpdate<Update<G::GameUpdate>>>();

                            let _ = ws_srv.do_send(ObserverMessage::<Request<G::GameRequest>, Update<G::GameUpdate>>::Register {
                                player_id,
                                observer: server_sender.clone(),
                            });

                            let keep_alive = tokio::time::sleep(TIMER_DURATION);
                            let mut last_activity = Instant::now();
                            let mut rate_limiter = RateLimiterState::default();
                            let mut measure_rtt_ping_governor = RateLimiterState::default();
                            const RATE: RateLimiterProps = RateLimiterProps::const_new(Duration::from_millis(80), 5);
                            const MEASURE_RTT_PING: RateLimiterProps = RateLimiterProps::const_new(Duration::from_secs(60), 0);

                            pin_mut!(keep_alive);

                            // For signaling what type of close frame should be sent, if any.
                            // See https://github.com/tokio-rs/axum/issues/1061
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
                                                                    let _ = ws_srv.do_send(ObserverMessage::<Request<G::GameRequest>, Update<G::GameUpdate >>::Request {
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
                                                            if !ALLOW_WEB_SOCKET_JSON.load(Ordering::Relaxed) || rate_limiter.should_limit_rate_with_now(&RATE, last_activity) {
                                                                continue;
                                                            }

                                                            let result: Result<Request<G::GameRequest>, serde_json::Error> = serde_json::from_str(&text);
                                                            match result {
                                                                Ok(request) => {
                                                                    protocol = WebSocketProtocol::Json;
                                                                    let _ = ws_srv.do_send(ObserverMessage::<Request<G::GameRequest>, Update<G::GameUpdate >>::Request {
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
                                                            // Axum spec says that automatic Pong will be sent.
                                                        }
                                                        Message::Pong(pong_data) => {
                                                            if rate_limiter.should_limit_rate_with_now(&RATE, last_activity) {
                                                                continue;
                                                            }

                                                            if let Ok(bytes) = pong_data.try_into() {
                                                                let now = get_unix_time_now();
                                                                let timestamp = UnixTime::from_ne_bytes(bytes);
                                                                let rtt = now.saturating_sub(timestamp);
                                                                if rtt <= 10000 as UnixTime {
                                                                    let _ = ws_srv.do_send(ObserverMessage::<Request<G::GameRequest>, Update<G::GameUpdate >>::RoundTripTime {
                                                                        player_id,
                                                                        rtt: rtt as u16,
                                                                    });
                                                                }
                                                            } else {
                                                                debug!("received invalid pong data");
                                                            }
                                                        },
                                                        Message::Close(_) => {
                                                            debug!("received close from client");
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
                                                if !ALLOW_WEB_SOCKET_JSON.load(Ordering::Relaxed) {
                                                    protocol = WebSocketProtocol::Binary;
                                                }
                                                let web_socket_message = match protocol {
                                                    WebSocketProtocol::Binary => Message::Binary(bincode::serialize(&message).unwrap()),
                                                    WebSocketProtocol::Json => Message::Text(serde_json::to_string(&message).unwrap()),
                                                };
                                                if web_socket.send(web_socket_message).await.is_err() {
                                                    break NORMAL_CLOSURE;
                                                }

                                                if !measure_rtt_ping_governor.should_limit_rate_with_now(&MEASURE_RTT_PING, last_activity) {
                                                    if web_socket.send(Message::Ping(get_unix_time_now().to_ne_bytes().into())).await.is_err() {
                                                        break NORMAL_CLOSURE;
                                                    }
                                                }
                                            }
                                            ObserverUpdate::Close => {
                                                break NORMAL_CLOSURE;
                                            }
                                        }
                                    },
                                    _ = keep_alive.as_mut() => {
                                        if last_activity.elapsed() < WEBSOCKET_HARD_TIMEOUT {
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

                            let _ = ws_srv.do_send(ObserverMessage::<Request<G::GameRequest>, Update<G::GameUpdate>>::Unregister {
                                player_id,
                                observer: server_sender,
                            });

                            if let Some(code) = closure {
                                let _ = web_socket.send(Message::Close(Some(CloseFrame{code, reason: "".into()}))).await;
                            } else {
                                let _ = web_socket.flush().await;
                            }
                        })),
                    },
                }
            }))
            .route("/system.json", axum::routing::get(move |ConnectInfo(addr): ConnectInfo<SocketAddr>, query: Query<SystemQuery>| {
                let srv = system_srv.to_owned();
                debug!("received system request");

                async move {
                    match srv
                        .send(SystemRequest {
                            ip: addr.ip(),
                            server_id: query.server_id,
                            region_id: query.region_id,
                            invitation_id: query.invitation_id,
                        })
                        .await
                    {
                        Ok(system_response) => {
                            Ok(Json(system_response))
                        }
                        Err(e) => {
                            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
                        }
                    }
                }
            }))
            .layer(axum::middleware::from_fn(async move |request: axum::http::Request<_>, next: axum::middleware::Next<_>| {
                let raw_path = request.uri().path();
                // The unwrap_or is purely defensive and should never happen.
                let path = raw_path.split('#').next().unwrap_or(raw_path);

                // We want to redirect everything except index.html (at any path level) so the
                // browser url-bar remains intact.
                let redirect = !path.is_empty() && !path.ends_with('/');

                if redirect {
                    if let Some((domain, server_id)) = domain
                        .as_ref()
                        .zip(ServerId::new(REDIRECT_TO_SERVER_ID.load(Ordering::Relaxed)))
                    {
                        let scheme = request.uri().scheme().cloned().unwrap_or(Scheme::HTTPS);
                        if let Ok(authority) = Authority::from_str(&format!("{}.{}", server_id.0.get(), domain)) {
                            let mut builder =  Uri::builder()
                                .scheme(scheme)
                                .authority(authority);

                            if let Some(path_and_query) = request.uri().path_and_query() {
                                builder = builder.path_and_query(path_and_query.clone());
                            }

                            if let Ok(uri) = builder.build() {
                                return Err(Redirect::temporary(&uri.to_string()));
                            }
                        }
                    }
                }

                Ok(next.run(request).await)
            }))
            .route("/leaderboard.json", get(move || {
                let srv = leaderboard_srv.to_owned();
                debug!("received status request");

                async move {
                    match srv.send(LeaderboardRequest).await {
                        Ok(leaderboard_response) => {
                            Ok(Json(leaderboard_response))
                        }
                        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
                    }
                }
            }))
            .route("/status.json", get(move || {
                let srv = status_srv.to_owned();
                debug!("received status request");

                async move {
                    match srv.send(StatusRequest).await {
                        Ok(status_response) => {
                            Ok(Json(status_response))
                        }
                        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
                    }
                }
            }))
            .route("/admin/", admin_router.clone())
            .route("/admin/*path", admin_router)
            .layer(ServiceBuilder::new()
                .layer(CorsLayer::new()
                    .allow_origin(tower_http::cors::AllowOrigin::predicate(move |origin, _parts| {
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
                    .allow_methods([Method::GET, Method::HEAD, Method::POST, Method::OPTIONS]))
                .layer(axum::middleware::from_fn(async move |request: axum::http::Request<_>, next: axum::middleware::Next<_>| {
                    let addr = request.extensions().get::<ConnectInfo<SocketAddr>>().map(|ci| ci.0);

                    if !request.headers().get("auth").map(|hv| constant_time_eq::constant_time_eq(include_str!("auth.txt").as_bytes(), hv.as_bytes())).unwrap_or(false) {
                        if let Err(response) = limit_content_length(request.headers(), 16384) {
                            return Err(response);
                        }
                    }

                    let ip = addr.map(|addr| addr.ip());
                    let mut response = next.run(request).await;

                    // Add some universal default headers.
                    let cross_origin_opener_policy = HeaderName::from_static("cross-origin-opener-policy");
                    for (key, value) in [(CACHE_CONTROL, "no-cache"), (cross_origin_opener_policy, "same-origin")] {
                        if !response.headers().contains_key(key.clone()) {
                            response.headers_mut()
                                .insert(key, HeaderValue::from_static(value));
                        }
                    }

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
                                boxed(Empty::new())
                            });
                        }
                    }

                    Ok(response)
                }))
            )
            // We limit even further later on.
            .layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024));

        let addr_incoming_config = axum_server::AddrIncomingConfig::new()
            .tcp_keepalive(Some(Duration::from_secs(32)))
            .tcp_nodelay(true)
            .tcp_sleep_on_accept_errors(true)
            .build();

        let http_config = axum_server::HttpConfig::new()
            .http1_keep_alive(true)
            .http1_header_read_timeout(Duration::from_secs(5))
            .max_buf_size(32768)
            .http2_max_concurrent_streams(Some(8))
            .http2_keep_alive_interval(Some(Duration::from_secs(4)))
            .http2_keep_alive_timeout(Duration::from_secs(10))
            .build();

        #[cfg(not(debug_assertions))]
        let http_app = Router::new()
            .fallback_service(get(async move |uri: Uri, host: TypedHeader<axum::headers::Host>, headers: reqwest::header::HeaderMap| {
                if let Err(response) = limit_content_length(&headers, 16384) {
                    return Err(response);
                }

                let mut parts = uri.into_parts();
                parts.scheme = Some(Scheme::HTTPS);
                let authority = if https_port == Options::STANDARD_HTTPS_PORT {
                    Authority::from_str(host.0.hostname())
                } else {
                    // non-standard port.
                    Authority::from_str(&format!("{}:{}", host.0.hostname(), https_port))
                }.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
                parts.authority = Some(authority);
                Uri::from_parts(parts)
                    .map(|uri| if http_port == Options::STANDARD_HTTP_PORT { Redirect::permanent(&uri.to_string()) } else { Redirect::temporary(&uri.to_string()) })
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
            }));

        #[cfg(debug_assertions)]
        let http_app = app;

        let http_server = axum_server::bind(SocketAddr::from(([0, 0, 0, 0], http_port)))
            .addr_incoming_config(addr_incoming_config.clone())
            .http_config(http_config.clone())
            .serve(http_app.into_make_service_with_connect_info::<SocketAddr>());

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

                let mut governor = tokio::time::interval(Duration::from_secs(24 * 60 * 60));

                loop {
                    // Every day.
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
        let https_server = axum_server::bind_rustls(SocketAddr::from(([0, 0, 0, 0], https_port)), rustls_config)
            .addr_incoming_config(addr_incoming_config.clone())
            .http_config(http_config)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>());

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
