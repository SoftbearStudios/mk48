// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(async_closure)]

use actix::prelude::*;
use actix_files as fs;
use actix_web::dev::Server;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use core::client::ParametrizedClientRequest;
use core::core::Core;
use core::server::ParametrizedServerRequest;
use core_protocol::rpc::{ClientRequest, ClientUpdate, ServerRequest, ServerUpdate};
use servutil::web_socket;

fn main() {
    actix_web::rt::System::new().block_on(async {
        std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
        env_logger::init();

        server().await.0
    });
}

pub async fn server() -> (Server, Addr<Core>) {
    let core = Core::start(Core::new(None, false).await);
    let core_clone = core.to_owned();

    (
        HttpServer::new(move || {
            let core_clone_1 = core_clone.to_owned();
            let core_clone_2 = core_clone.to_owned();
            let core_clone_3 = core_clone.to_owned();
            let core_clone_4 = core_clone.to_owned();

            App::new()
                .wrap(middleware::Logger::default())
                .service(web::resource("/client/ws/").route(web::get().to(
                    move |r: HttpRequest, stream: web::Payload| {
                        web_socket::sock_index::<Core, ClientRequest, ClientUpdate>(
                            r,
                            stream,
                            core_clone_1.to_owned(),
                        )
                    },
                )))
                .service(web::resource("/client/").route(web::post().to(
                    move |request: web::Json<ParametrizedClientRequest>| {
                        let core = core_clone_2.to_owned();

                        async move {
                            match core.send(request.0).await {
                                Ok(result) => match result {
                                    actix_web::Result::Ok(update) => {
                                        let response = serde_json::to_vec(&update).unwrap();
                                        HttpResponse::Ok().body(response)
                                    }
                                    Err(e) => HttpResponse::BadRequest().body(String::from(e)),
                                },
                                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                            }
                        }
                    },
                )))
                .service(web::resource("/server/ws/").route(web::get().to(
                    move |r: HttpRequest, stream: web::Payload| {
                        web_socket::sock_index::<Core, ServerRequest, ServerUpdate>(
                            r,
                            stream,
                            core_clone_3.to_owned(),
                        )
                    },
                )))
                .service(web::resource("/server/").route(web::post().to(
                    move |request: web::Json<ParametrizedServerRequest>| {
                        let core = core_clone_4.to_owned();

                        async move {
                            match core.send(request.0).await {
                                Ok(result) => match result {
                                    actix_web::Result::Ok(update) => {
                                        let response = serde_json::to_vec(&update).unwrap();
                                        HttpResponse::Ok().body(response)
                                    }
                                    Err(e) => HttpResponse::BadRequest().body(String::from(e)),
                                },
                                Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                            }
                        }
                    },
                )))
                .service(fs::Files::new("/", "static/").index_file("index.html"))
        })
        .bind("0.0.0.0:8192")
        .expect("could not listen at port 8192")
        .run(),
        core,
    )
}
