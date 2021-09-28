// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::observer::*;
use actix::dev::ToEnvelope;
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use actix_web_actors::ws::{CloseCode, CloseReason};
use core_protocol::web_socket::WebSocketFormat;
use log::{debug, warn};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::{Duration, Instant};

const TIMER_SECONDS: u64 = 30;
pub const TIMER_DURATION: Duration = Duration::from_secs(TIMER_SECONDS);
pub const WEBSOCK_SOFT_TIMEOUT: Duration = Duration::from_secs(TIMER_SECONDS * 4 / 5);
pub const WEBSOCK_HARD_TIMEOUT: Duration = Duration::from_secs(TIMER_SECONDS * 3);

pub async fn sock_index<A, I, O>(
    r: HttpRequest,
    stream: web::Payload,
    data: Addr<A>,
) -> Result<HttpResponse, Error>
where
    A: Handler<ObserverMessage<I, O>>,
    <A as Actor>::Context: ToEnvelope<A, ObserverMessage<I, O>>,
    I: 'static + Send + DeserializeOwned,
    O: 'static + Message + Send + Serialize,
    O: Message<Result = ()>,
{
    ws::start(
        WebSocket::<I, O>::new(data.recipient(), WebSocketFormat::Json, ()),
        &r,
        stream,
    )
}

pub struct WebSocket<I, O, P = ()>
where
    I: 'static + Send,
    O: 'static + Message + Send + Serialize,
    O: Message<Result = ()>,
    P: 'static + Clone + Send + Unpin,
{
    format: WebSocketFormat,
    data: Recipient<ObserverMessage<I, O, P>>,
    date_last_activity: Instant,
    payload: P,
}

impl<I, O, P> WebSocket<I, O, P>
where
    I: 'static + Send,
    O: 'static + Message + Send + Serialize,
    O: Message<Result = ()>,
    P: 'static + Clone + Send + Unpin,
{
    pub fn new(
        data: Recipient<ObserverMessage<I, O, P>>,
        format: WebSocketFormat,
        payload: P,
    ) -> Self {
        Self {
            format,
            data,
            date_last_activity: Instant::now(),
            payload,
        }
    }

    fn set_keep_alive(&mut self) {
        self.date_last_activity = Instant::now();
    }

    fn start_timer(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(TIMER_DURATION, |act, ctx| {
            let elapsed = Instant::now().duration_since(act.date_last_activity);
            if elapsed > WEBSOCK_HARD_TIMEOUT {
                warn!("disconnect unresponsive websocket");
                ctx.close(None);
                ctx.stop();
            } else if elapsed > WEBSOCK_SOFT_TIMEOUT {
                debug!("ping idle websocket");
                ctx.ping(b"");
            }
        });
    }
}

impl<I, O, P> Actor for WebSocket<I, O, P>
where
    I: 'static + Send,
    O: 'static + Message + Serialize + Send,
    O: Message<Result = ()>,
    P: 'static + Clone + Send + Unpin,
{
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let _ = self.data.do_send(ObserverMessage::<I, O, P>::Register {
            observer: ctx.address().recipient(),
            payload: self.payload.clone(),
        });

        self.start_timer(ctx);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        let _ = self.data.do_send(ObserverMessage::<I, O, P>::Unregister {
            observer: ctx.address().recipient(),
        });
    }
}

impl<I, O, P> Handler<ObserverUpdate<O>> for WebSocket<I, O, P>
where
    I: 'static + Send,
    O: 'static + Message<Result = ()> + Send + Serialize,
    P: 'static + Clone + Send + Unpin,
{
    type Result = O::Result;

    fn handle(&mut self, update: ObserverUpdate<O>, ctx: &mut Self::Context) {
        match update {
            ObserverUpdate::Send { message } => match self.format {
                WebSocketFormat::Binary => ctx.binary(bincode::serialize(&message).unwrap()),
                WebSocketFormat::Json => ctx.text(serde_json::to_string(&message).unwrap()),
            },
            ObserverUpdate::Close => ctx.close(Some(CloseReason::from(CloseCode::Normal))),
        }
    }
}

impl<I, O, P> StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocket<I, O, P>
where
    I: 'static + Send + DeserializeOwned,
    O: 'static + Message<Result = ()> + Send + Serialize,
    P: 'static + Clone + Send + Unpin,
{
    fn handle(
        &mut self,
        ws_message: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        self.set_keep_alive();
        match ws_message {
            Ok(ws::Message::Binary(bin)) => {
                match bincode::deserialize(bin.as_ref()) {
                    Ok(request) => {
                        self.format = WebSocketFormat::Binary;
                        let _ = self.data.do_send(ObserverMessage::<I, O, P>::Request {
                            observer: ctx.address().recipient(),
                            request,
                        });
                    }
                    Err(err) => {
                        warn!("deserialize binary err ignored {}", err);
                    }
                } // match result
            }
            Ok(ws::Message::Close(_reason)) => {
                debug!("close websocket request");
                //ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Ping(ping_data)) => {
                debug!("received ping");
                ctx.pong(&ping_data);
            }
            Ok(ws::Message::Pong(_)) => {
                debug!("received pong");
                // set_keep_alive already called
            }
            Ok(ws::Message::Text(text)) => {
                debug!("request {}", text);
                let result: Result<I, serde_json::Error> = serde_json::from_str(&text);
                match result {
                    Ok(request) => {
                        self.format = WebSocketFormat::Json;
                        let _ = self.data.do_send(ObserverMessage::<I, O, P>::Request {
                            observer: ctx.address().recipient(),
                            request,
                        });
                    }
                    Err(err) => {
                        warn!("parse err ignored {}", err);
                    }
                } // match result
            }
            _ => {
                warn!("websocket protocol error");
                //ctx.close("protocol error")
                ctx.stop()
            }
        }
    }
}
