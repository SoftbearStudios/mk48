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
use actix_tls::accept::rustls::TlsStream;
use actix_web::dev::Service;
use actix_web::http::header::CACHE_CONTROL;
use actix_web::http::Version;
use actix_web::middleware::DefaultHeaders;
use actix_web::rt::net::TcpStream;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use actix_web_middleware_redirect_https::RedirectHTTPS;
use common::entity::EntityType;
use common::protocol::{Command, Update};
use connection_leak_detector::{ConnectionLeakDetector, Encryption, Protocol, Verdict};
use core::app::core_services;
use core_protocol::dto::InvitationDto;
use core_protocol::get_unix_time_now;
use core_protocol::id::*;
use core_protocol::web_socket::WebSocketFormat;
use env_logger;
use lazy_static::lazy_static;
use log::{error, warn, LevelFilter};
use serde::Deserialize;
use servutil::app::{include_dir, static_files};
use servutil::cloud::Cloud;
use servutil::linode::Linode;
use servutil::ssl::{run_until_ssl_renewal, Ssl};
use servutil::tcp::{
    max_connections_per_worker, on_connect_enable_nodelay, BACKLOG, KEEP_ALIVE, SHUTDOWN_TIMEOUT,
};
use servutil::watchdog;
use servutil::web_socket::WebSocket;
use std::fs::OpenOptions;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;
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
    /// Verbosity
    #[structopt(short, long, parse(from_occurrences))]
    verbose: usize,
    /// Log incoming HTTP requests
    #[structopt(long)]
    debug_http: bool,
    /// Log game diagnostics
    #[structopt(long)]
    debug_game: bool,
    /// Log core diagnostics
    #[structopt(long)]
    debug_core: bool,
    /// Log socket diagnostics
    #[structopt(long)]
    debug_sockets: bool,
    /// Log watchdog diagnostics
    #[structopt(long)]
    debug_watchdog: bool,
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

lazy_static! {
    pub static ref CONNECTION_LEAK_DETECTOR: Mutex<ConnectionLeakDetector> = Mutex::new({
        let mut detector = ConnectionLeakDetector::new();
        detector.set_log_path("/tmp/cld.csv");
        detector
    });
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

fn main() {
    // SAFETY: As per spec, only called once (before .data()) is called.
    unsafe {
        EntityType::init();
        noise::init()
    }

    let options = Options::from_args();

    let mut logger = env_logger::builder();
    logger.format_timestamp(None);
    let level = match options.verbose {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    if options.debug_game {
        logger.filter_module(module_path!(), level);
    }
    if options.debug_core {
        logger.filter_module("core", level);
        logger.filter_module("core_protocol", level);
    }
    if options.debug_sockets {
        logger.filter_module("servutil::web_socket", level);
    }
    if options.debug_http {
        logger.filter_module("actix_web", LevelFilter::Info);
        logger.filter_module("actix_server", LevelFilter::Info);
    }
    if options.debug_watchdog || true {
        logger.filter_module("servutil::watchdog", LevelFilter::Info);
        logger.filter_module("servutil::linode", LevelFilter::Warn);
    }
    logger.init();

    let _ = actix_web::rt::System::new().block_on(async move {
        let (cld_send, mut cld_recv) = lockfree::channel::mpsc::create::<(
            SocketAddr,
            Option<Protocol>,
            Option<Encryption>,
            Option<Verdict>,
        )>();

        actix_web::rt::spawn(async move {
            let mut i = 3600;
            loop {
                {
                    let mut cld = CONNECTION_LEAK_DETECTOR.lock().unwrap();

                    while let Ok((addr, protocol, encryption, verdict)) = cld_recv.recv() {
                        cld.mark_connection(&addr, protocol, encryption, verdict);
                    }

                    if i % 60 == 0 {
                        if let Err(e) = cld.update() {
                            error!("CLD error: {:?}", e);
                        }
                    }

                    i += 1;

                    if i >= 3600 {
                        let leaked: Vec<_> = cld.iter_leaked_connections().collect();
                        match serde_json::to_string(&leaked) {
                            Ok(serialized) => {
                                match OpenOptions::new()
                                    .create(true)
                                    .write(true)
                                    .open(format!("/tmp/dat_cld_{}.csv", get_unix_time_now()))
                                {
                                    Ok(mut file) => {
                                        use std::io::Write;
                                        let _ = write!(file, "{}", serialized);
                                    }
                                    Err(e) => error!("couldn't open CLD data file: {:?}", e),
                                }
                            }
                            Err(e) => error!("couldn't serialize CLD data: {:?}", e),
                        }
                        i = 0;
                    }
                }
                actix_web::rt::time::sleep(Duration::from_secs(1)).await;
            }
        });

        let cloud = options
            .linode_personal_access_token
            .map(|t| Box::new(Linode::new(&t)) as Box<dyn Cloud>);

        let core = core::core::Core::start(
            core::core::Core::new(options.chat_log, options.database_read_only).await,
        );
        let srv = server::Server::start(server::Server::new(
            ServerId::new(options.server_id),
            options.min_players,
            core.to_owned(),
        ));
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
                Ssl::new(&certificate_file, &private_key_file).unwrap()
            });

        let use_ssl = ssl.is_some();

        loop {
            let iter_core = core.to_owned();
            let iter_srv = srv.to_owned();

            // If ssl exists, safe to assume whatever certificates exist are now installed.
            ssl.as_mut().map(|ssl| ssl.set_renewed());
            let immut_ssl = &ssl;

            // First clone due to loop.
            let cld_send = cld_send.clone();
            let cld_send_on_connect = cld_send.clone();

            let mut server = HttpServer::new(move || {
                // Rust let's you get away with cloning one closure deep, not all the way to a nested closure.
                let core_clone = iter_core.to_owned();
                let srv_clone = iter_srv.to_owned();
                let cld_send = cld_send.clone();

                App::new()
                    .wrap_fn(move |req, srv| {
                        if let Some(addr) = req
                            .connection_info()
                            .remote_addr()
                            .and_then(|s| SocketAddr::from_str(s).ok())
                        {
                            let mut protocol = match req.version() {
                                Version::HTTP_09 => Protocol::Http09,
                                Version::HTTP_10 => Protocol::Http10,
                                Version::HTTP_11 => Protocol::Http11,
                                Version::HTTP_2 => Protocol::Http2,
                                Version::HTTP_3 => Protocol::Http3,
                                _ => Protocol::Tcp,
                            };
                            if let Some(upgrade) = req.headers().get("upgrade") {
                                if upgrade == "websocket" {
                                    protocol = Protocol::WebSocket;
                                }
                            }
                            let encryption = match req.connection_info().scheme() {
                                "http" | "ws" => Encryption::None,
                                "https" | "wss" => Encryption::Tls,
                                _ => Encryption::Unknown,
                            };
                            let _ = cld_send.send((
                                addr,
                                Some(protocol),
                                Some(encryption),
                                Some(Verdict::MadeRequest),
                            ));
                        } else {
                            warn!("Ghost connection (no remote address)");
                        }
                        srv.call(req)
                    })
                    .wrap(RedirectHTTPS::default().set_enabled(use_ssl))
                    .wrap(middleware::Logger::default())
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
                    .wrap(DefaultHeaders::new().header(CACHE_CONTROL, "no-cache"))
                    .configure(static_files(&include_dir!("../js/public")))
            })
            .on_connect(move |conn, ext| {
                let (plain, encryption) =
                    if let Some(rustls) = conn.downcast_ref::<TlsStream<TcpStream>>() {
                        (rustls.get_ref().0, Encryption::Tls)
                    } else if let Some(plain) = conn.downcast_ref::<TcpStream>() {
                        (plain, Encryption::None)
                    } else {
                        debug_assert!(false);
                        return;
                    };

                if let Ok(addr) = plain.peer_addr() {
                    let _ = cld_send_on_connect.send((
                        addr,
                        None,
                        Some(encryption),
                        Some(Verdict::New),
                    ));
                }

                on_connect_enable_nodelay(conn, ext);
            });

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

            if let Err(_) = run_until_ssl_renewal(running_server, immut_ssl).await {
                break;
            }
        }
    });
}
