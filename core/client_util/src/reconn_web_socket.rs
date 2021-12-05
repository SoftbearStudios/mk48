// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::web_socket::{ProtoWebSocket, State};
use core_protocol::web_socket::WebSocketFormat;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Reconnectable WebSocket.
pub struct ReconnWebSocket<I, O> {
    inner: ProtoWebSocket<I, O>,
    host: String,
    /// Send when opening a new socket.
    preamble: Option<O>,
    tries: u8,
    next_try: f32,
}

impl<I, O> ReconnWebSocket<I, O>
where
    I: 'static + DeserializeOwned,
    O: 'static + Serialize + Clone,
{
    const MAX_TRIES: u8 = 5;
    const SECONDS_PER_TRY: f32 = 1.0;

    pub fn new(host: &str, format: WebSocketFormat, preamble: Option<O>) -> Self {
        let mut inner = ProtoWebSocket::new(host, format);

        preamble.as_ref().map(|p| inner.send(p.clone()));

        Self {
            inner,
            preamble,
            host: String::from(host),
            tries: 0,
            next_try: 0.0,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    pub fn reconnect_if_necessary(&mut self, time_seconds: f32) {
        //crate::console_log!("status: {:?}, tries: {}, until: {}", self.inner.state(), self.tries, self.next_try - time_seconds);

        if self.inner.state() == State::Open {
            // Reconnected, forget tries.
            self.tries = 0;
            self.next_try = time_seconds + Self::SECONDS_PER_TRY * 0.5;
        } else if self.inner.is_error()
            && self.tries < Self::MAX_TRIES
            && time_seconds > self.next_try
        {
            // Try again.
            self.inner = ProtoWebSocket::new(&self.host, self.inner.format());
            self.preamble.clone().map(|p| self.inner.send(p));
            self.tries += 1;
            self.next_try = time_seconds + Self::SECONDS_PER_TRY;
        }
    }

    pub fn set_format(&mut self, format: WebSocketFormat) {
        self.inner.set_format(format);
    }

    pub fn receive_updates(&mut self) -> Vec<I> {
        self.inner.receive_updates()
    }

    pub fn send(&mut self, msg: O) {
        self.inner.send(msg);
    }

    /// Drop, but leave open the possibility of auto-reconnecting (useful for testing Self).
    pub fn drop(&mut self) {
        self.inner.close();
    }

    pub fn close(&mut self) {
        self.inner.close();

        // Close was intentional, so ensure no reconnect is attempted.
        self.tries = Self::MAX_TRIES;
    }
}
