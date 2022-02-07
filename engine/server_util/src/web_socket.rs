// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::observer::*;
use crate::rate_limiter::{RateLimiter, RateLimiterProps};
use crate::user_agent::UserAgent;
use actix::dev::ToEnvelope;
use actix::prelude::*;
use actix_web::http::header;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use actix_web_actors::ws::{CloseCode, CloseReason};
use bincode::Options;
use core_protocol::web_socket::WebSocketFormat;
use core_protocol::{get_unix_time_now, UnixTime};
use log::{debug, info, warn};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::convert::TryInto;
use std::net::IpAddr;
use std::time::{Duration, Instant};

const TIMER_SECONDS: u64 = 5;
pub const TIMER_DURATION: Duration = Duration::from_secs(TIMER_SECONDS);
pub const WEBSOCK_SOFT_TIMEOUT: Duration = Duration::from_secs(TIMER_SECONDS * 4 / 5);
pub const WEBSOCK_HARD_TIMEOUT: Duration = Duration::from_secs(TIMER_SECONDS * 2);

pub async fn sock_index<A, I, O>(
    r: HttpRequest,
    stream: web::Payload,
    data: Addr<A>,
) -> Result<HttpResponse, Error>
where
    A: Handler<ObserverMessage<I, O, (Option<IpAddr>, Option<UserAgent>)>>,
    <A as Actor>::Context:
        ToEnvelope<A, ObserverMessage<I, O, (Option<IpAddr>, Option<UserAgent>)>>,
    I: 'static + Send + DeserializeOwned,
    O: 'static + Message + Send + Serialize,
    O: Message<Result = ()>,
{
    let ip_address = r.peer_addr().map(|addr| addr.ip());
    let user_agent = r
        .headers()
        .get(header::USER_AGENT)
        .and_then(|hv| hv.to_str().ok())
        .map(UserAgent::new);

    debug_assert!(ip_address.is_some());

    ws::start(
        WebSocket::<I, O, (Option<IpAddr>, Option<UserAgent>)>::new(
            data.recipient(),
            WebSocketFormat::Json,
            RateLimiterProps::new(Duration::from_millis(90), 5),
            (ip_address, user_agent),
        ),
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
    rate_limiter: RateLimiter,
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
        rate_limiter_props: RateLimiterProps,
        payload: P,
    ) -> Self {
        Self {
            format,
            data,
            date_last_activity: Instant::now(),
            payload,
            rate_limiter: RateLimiter::from(rate_limiter_props),
        }
    }

    fn set_keep_alive(&mut self) {
        self.date_last_activity = Instant::now();
    }

    fn start_timer(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(TIMER_DURATION, |act, ctx| {
            let elapsed = act.date_last_activity.elapsed();
            if elapsed > WEBSOCK_HARD_TIMEOUT {
                warn!(
                    "disconnect unresponsive websocket {:?} {:?}",
                    act.format, elapsed
                );
                ctx.close(None);
                ctx.stop();
            } else if elapsed > WEBSOCK_SOFT_TIMEOUT {
                warn!("ping idle websocket {:?} {:?}", act.format, elapsed);
                ctx.ping(&get_unix_time_now().to_ne_bytes());
            } else {
                info!("websocket is responsive {:?} {:?}", act.format, elapsed);
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

        info!("websocket started {:?}", self.format);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        let _ = self.data.do_send(ObserverMessage::<I, O, P>::Unregister {
            observer: ctx.address().recipient(),
        });

        info!("websocket stopped {:?}", self.format);
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
                if self
                    .rate_limiter
                    .should_limit_rate_with_now(self.date_last_activity)
                {
                    debug!("binary request rate limited");
                    return;
                }

                match bincode::DefaultOptions::new()
                    .with_limit(1024 * 1024)
                    .with_fixint_encoding()
                    .allow_trailing_bytes()
                    .deserialize(bin.as_ref())
                {
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
            Ok(ws::Message::Pong(pong_data)) => {
                if self
                    .rate_limiter
                    .should_limit_rate_with_now(self.date_last_activity)
                {
                    debug!("pong was rate limited");
                    return;
                }

                debug!("received pong");

                if let Ok(bytes) = pong_data.as_ref().try_into() {
                    let now = get_unix_time_now();
                    let timestamp = UnixTime::from_ne_bytes(bytes);
                    let rtt = now.saturating_sub(timestamp);
                    if rtt < u16::MAX as UnixTime {
                        let _ = self
                            .data
                            .do_send(ObserverMessage::<I, O, P>::RoundTripTime {
                                observer: ctx.address().recipient(),
                                rtt: rtt as u16,
                            });
                    }
                } else {
                    debug!("received invalid pong data");
                }
            }
            Ok(ws::Message::Text(text)) => {
                if self
                    .rate_limiter
                    .should_limit_rate_with_now(self.date_last_activity)
                {
                    debug!("text request was rate limited");
                    return;
                }

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
