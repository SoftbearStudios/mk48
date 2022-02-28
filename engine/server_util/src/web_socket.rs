// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::observer::*;
use crate::rate_limiter::{RateLimiter, RateLimiterProps};
use actix::prelude::*;
use actix_web_actors::ws;
use actix_web_actors::ws::{CloseCode, CloseReason};
use bincode::Options;
use core_protocol::id::PlayerId;
use core_protocol::web_socket::WebSocketProtocol;
use core_protocol::{get_unix_time_now, UnixTime};
use log::{debug, info, warn};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::convert::TryInto;
use std::time::{Duration, Instant};

const TIMER_SECONDS: u64 = 5;
pub const TIMER_DURATION: Duration = Duration::from_secs(TIMER_SECONDS);
pub const WEBSOCK_SOFT_TIMEOUT: Duration = Duration::from_secs(TIMER_SECONDS * 4 / 5);
pub const WEBSOCK_HARD_TIMEOUT: Duration = Duration::from_secs(TIMER_SECONDS * 2);

pub struct WebSocket<I, O, P = ()>
where
    I: 'static + Send,
    O: 'static + Message + Send + Serialize,
    O: Message<Result = ()>,
    P: 'static + Clone + Send + Unpin,
{
    format: WebSocketProtocol,
    data: Recipient<ObserverMessage<I, O, P>>,
    date_last_activity: Instant,
    player_id: PlayerId,
    payload: P,
    rate_limiter: RateLimiter,
    // How many more pings to test rtt.
    test_rtt: u8,
}

impl<I, O, P> WebSocket<I, O, P>
where
    I: 'static + Send,
    O: 'static + Message + Send + Serialize,
    O: Message<Result = ()>,
    P: 'static + Clone + Send + Unpin,
{
    const INBOUND_MESSAGE_MAX_BYTES: usize = 32 * 1024;

    pub fn new(
        data: Recipient<ObserverMessage<I, O, P>>,
        format: WebSocketProtocol,
        rate_limiter_props: RateLimiterProps,
        player_id: PlayerId,
        payload: P,
    ) -> Self {
        Self {
            format,
            data,
            date_last_activity: Instant::now(),
            player_id,
            payload,
            rate_limiter: RateLimiter::from(rate_limiter_props),
            test_rtt: 2,
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
            } else if elapsed > WEBSOCK_SOFT_TIMEOUT || act.test_rtt > 0 {
                if elapsed > WEBSOCK_SOFT_TIMEOUT {
                    warn!("ping idle websocket {:?} {:?}", act.format, elapsed);
                }
                act.test_rtt = act.test_rtt.saturating_sub(1);
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
            player_id: self.player_id,
            observer: ctx.address().recipient(),
            payload: self.payload.clone(),
        });

        self.start_timer(ctx);

        info!("websocket started {:?}", self.format);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        let _ = self.data.do_send(ObserverMessage::<I, O, P>::Unregister {
            player_id: self.player_id,
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
                WebSocketProtocol::Binary => ctx.binary(bincode::serialize(&message).unwrap()),
                WebSocketProtocol::Json => ctx.text(serde_json::to_string(&message).unwrap()),
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
                    .with_limit(Self::INBOUND_MESSAGE_MAX_BYTES as u64)
                    .with_fixint_encoding()
                    .allow_trailing_bytes()
                    .deserialize(bin.as_ref())
                {
                    Ok(request) => {
                        self.format = WebSocketProtocol::Binary;
                        let _ = self.data.do_send(ObserverMessage::<I, O, P>::Request {
                            player_id: self.player_id,
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
                                player_id: self.player_id,
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

                if text.len() > Self::INBOUND_MESSAGE_MAX_BYTES {
                    warn!(
                        "client text message was {} bytes, exceeding limit",
                        text.len()
                    );
                    return;
                }

                let result: Result<I, serde_json::Error> = serde_json::from_str(&text);
                match result {
                    Ok(request) => {
                        self.format = WebSocketProtocol::Json;
                        let _ = self.data.do_send(ObserverMessage::<I, O, P>::Request {
                            player_id: self.player_id,
                            request,
                        });
                    }
                    Err(err) => {
                        warn!("parse err ignored {}", err);
                    }
                } // match result
            }
            Ok(ws::Message::Nop) => {
                // Ignore.
            }
            _ => {
                warn!("websocket protocol error");
                //ctx.close("protocol error")
                ctx.stop()
            }
        }
    }
}
