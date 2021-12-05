use crate::admin::*;
use crate::client::*;
use crate::core::Core;
use actix::Addr;
use actix_web::web::Payload;
use actix_web::web::{get, post, resource, Json, ServiceConfig};
use actix_web::{HttpRequest, HttpResponse};
use core_protocol::rpc::*;
use log::debug;
use servutil::web_socket::sock_index;

pub fn core_services(core: Addr<Core>) -> impl Fn(&mut ServiceConfig) {
    move |cfg: &mut ServiceConfig| {
        let client_ws_core = core.clone();
        let client_core = core.clone();
        let admin_core = core.clone();
        let status_core = core.clone();

        cfg.service(resource("/client/ws/").route(get().to(
            move |r: HttpRequest, stream: Payload| {
                sock_index::<Core, ClientRequest, ClientUpdate>(
                    r,
                    stream,
                    client_ws_core.to_owned(),
                )
            },
        )))
        .service(resource("/client/").route(post().to(
            move |request: Json<ParametrizedClientRequest>| {
                let core = client_core.to_owned();
                debug!("received client request");
                // HttpResponse impl's Future, but that is irrelevant in this context.
                #[allow(clippy::async_yields_async)]
                async move {
                    match core.send(request.0).await {
                        Ok(result) => match result {
                            actix_web::Result::Ok(update) => {
                                let response = serde_json::to_vec(&update).unwrap();
                                HttpResponse::Ok()
                                    .content_type("application/json")
                                    .body(response)
                            }
                            Err(e) => HttpResponse::BadRequest().body(String::from(e)),
                        },
                        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                    }
                }
            },
        )))
        .service(resource("/admin/").route(post().to(
            move |request: Json<ParameterizedAdminRequest>| {
                let core = admin_core.to_owned();
                debug!("received metric request");
                // HttpResponse impl's Future, but that is irrelevant in this context.
                #[allow(clippy::async_yields_async)]
                async move {
                    match core.send(request.0).await {
                        Ok(result) => match result {
                            actix_web::Result::Ok(update) => {
                                let response = serde_json::to_vec(&update).unwrap();
                                HttpResponse::Ok()
                                    .content_type("application/json")
                                    .body(response)
                            }
                            Err(e) => HttpResponse::BadRequest().body(String::from(e)),
                        },
                        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                    }
                }
            },
        )))
        .service(resource("/status/").route(get().to(move || {
            let core = status_core.to_owned();
            debug!("received status request");
            let request = ParameterizedAdminRequest {
                params: AdminState {
                    auth: AdminState::AUTH.to_string(),
                },
                request: AdminRequest::RequestStatus,
            };
            // HttpResponse impl's Future, but that is irrelevant in this context.
            #[allow(clippy::async_yields_async)]
            async move {
                match core.send(request).await {
                    Ok(result) => match result {
                        actix_web::Result::Ok(update) => {
                            let response = serde_json::to_vec(&update).unwrap();
                            HttpResponse::Ok()
                                .content_type("application/json")
                                .body(response)
                        }
                        Err(e) => HttpResponse::BadRequest().body(String::from(e)),
                    },
                    Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                }
            }
        })));
    }
}
