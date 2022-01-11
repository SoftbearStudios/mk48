// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apply::Apply;
use crate::web_socket::{ProtoWebSocket, State};
use core_protocol::web_socket::WebSocketFormat;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Reconnectable WebSocket (generic over inbound, outbound, and state).
/// Old state is preserved after closing, but cleared when a new connection is reopened.
pub struct ReconnWebSocket<I, O, S> {
    inner: ProtoWebSocket<I, O>,
    state: S,
    host: String,
    /// Tracks whether the socket was closed, so the state can be cleared as soon as it is reopened.
    was_closed: bool,
    /// Send when opening a new socket.
    preamble: Option<O>,
    tries: u8,
    next_try: f32,
}

impl<I, O, S> ReconnWebSocket<I, O, S>
where
    I: 'static + DeserializeOwned,
    O: 'static + Serialize + Clone,
    S: Apply<I>,
{
    const MAX_TRIES: u8 = 5;
    const SECONDS_PER_TRY: f32 = 1.0;

    pub fn new(host: &str, format: WebSocketFormat, preamble: Option<O>) -> Self {
        let mut inner = ProtoWebSocket::new(host, format);

        if let Some(p) = preamble.as_ref() {
            inner.send(p.clone());
        }

        Self {
            inner,
            state: Default::default(),
            preamble,
            host: String::from(host),
            was_closed: false,
            tries: 0,
            next_try: 0.0,
        }
    }

    /// Returns whether the underlying connection is closed (for any reason).
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Returns whether the underlying connection is open.
    pub fn is_open(&self) -> bool {
        self.inner.is_open()
    }

    /// Returns whether the underlying connection is closed and reconnection attempts have been
    /// exhausted.
    pub fn is_terminated(&self) -> bool {
        self.inner.is_error() && self.tries >= Self::MAX_TRIES
    }

    /// Takes the current time, and returns a collection of updates to apply to the current
    /// state. Will automatically reconnect and clear state if/when the underlying connection is new.
    ///
    /// TODO: Until further notice, it is the caller's responsibility to apply the state changes.
    pub fn update(&mut self, time_seconds: f32) -> Vec<I> {
        if self.is_closed() {
            self.was_closed = true;
        } else if self.was_closed && self.is_open() && self.inner.has_updates() {
            self.was_closed = false;
            // Need to clear state, since websocket is *no longer* closed and has new updates.
            self.state = S::default();
        }

        self.reconnect_if_necessary(time_seconds);
        self.inner.receive_updates()
    }

    /// Reset the preamble to a different value.
    pub fn reset_preamble(&mut self, preamble: O) {
        self.preamble = Some(preamble);
    }

    /// Sets the format that will be used to send subsequent messages.
    pub fn set_format(&mut self, format: WebSocketFormat) {
        self.inner.set_format(format);
    }

    /// Sends a message, or queues it for sending when the underlying connection is open.
    pub fn send(&mut self, msg: O) {
        self.inner.send(msg);
    }

    /// Immutable reference to corresponding state, which is reset when a new connection is established.
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Mutable reference to corresponding state, which is reset when a new connection is established.
    pub fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }

    /// Attempts to reestablish a connection if necessary. This does not and should not preserve
    /// pending messages.
    fn reconnect_if_necessary(&mut self, time_seconds: f32) {
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
            if let Some(p) = self.preamble.as_ref() {
                self.inner.send(p.clone());
            }
            self.tries += 1;
            self.next_try = time_seconds + Self::SECONDS_PER_TRY;
        } else if self.tries >= Self::MAX_TRIES {
            // Stop trying, stop giving the impression of working.
            self.state = S::default();
        }
    }

    /// Drop, but leave open the possibility of auto-reconnecting (useful for testing Self).
    pub fn simulate_drop(&mut self) {
        self.inner.close();
    }
}

impl<I, O, S> Drop for ReconnWebSocket<I, O, S> {
    fn drop(&mut self) {
        self.inner.close();
    }
}
