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
use actix_web::dev::{Service, Url};
use actix_web::http::header::{ACCEPT, CACHE_CONTROL};
use actix_web::http::uri::PathAndQuery;
use actix_web::http::Uri;
use actix_web::middleware::DefaultHeaders;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use actix_web_middleware_redirect_https::RedirectHTTPS;
use common::entity::EntityType;
use common::protocol::{Command, Update};
use core::app::core_services;
use core_protocol::dto::InvitationDto;
use core_protocol::id::*;
use core_protocol::web_socket::WebSocketFormat;
use env_logger;
use log::LevelFilter;
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
        noise::init();

        for typ in EntityType::iter() {
            rustrict::add_word(typ.to_str(), rustrict::Type::SAFE);
        }
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
        logger.filter_module("servutil::ssl", LevelFilter::Info);
    }
    logger.init();

    let _ = actix_web::rt::System::new().block_on(async move {
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

            let mut server = HttpServer::new(move || {
                // Rust let's you get away with cloning one closure deep, not all the way to a nested closure.
                let core_clone = iter_core.to_owned();
                let srv_clone = iter_srv.to_owned();

                App::new()
                    .wrap_fn(move |mut req, srv| {
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

            if let Err(_) = run_until_ssl_renewal(running_server, immut_ssl).await {
                break;
            }
        }
    });
}
