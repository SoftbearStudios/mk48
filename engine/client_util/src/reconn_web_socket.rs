// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apply::Apply;
use crate::web_socket::{ProtoWebSocket, State};
use core_protocol::web_socket::WebSocketProtocol;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

/// Reconnectable WebSocket (generic over inbound, outbound, and state).
/// Old state is preserved after closing, but cleared when a new connection is reopened.
pub struct ReconnWebSocket<I, O, S> {
    inner: ProtoWebSocket<I, O>,
    host: String,
    /// Tracks whether the socket was closed, so the state can be cleared as soon as it is reopened.
    was_closed: bool,
    /// Send when opening a new socket.
    preamble: Option<O>,
    tries: u8,
    next_try: f32,
    _spooky: PhantomData<S>,
}

impl<I, O, S> ReconnWebSocket<I, O, S>
where
    I: 'static + DeserializeOwned,
    O: 'static + Serialize + Clone,
    S: Apply<I>,
{
    const MAX_TRIES: u8 = 5;
    const SECONDS_PER_TRY: f32 = 1.0;

    pub fn new(host: String, protocol: WebSocketProtocol, preamble: Option<O>) -> Self {
        let mut inner = ProtoWebSocket::new(&host, protocol);

        if let Some(p) = preamble.as_ref() {
            inner.send(p.clone());
        }

        Self {
            inner,
            preamble,
            host,
            was_closed: false,
            tries: 0,
            next_try: 0.0,
            _spooky: PhantomData,
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

    pub fn is_reconnecting(&self) -> bool {
        matches!(self.inner.state(), State::Opening | State::Error)
            && (1..=Self::MAX_TRIES).contains(&self.tries)
    }

    /// Returns whether the underlying connection is closed and reconnection attempts have been
    /// exhausted.
    pub fn is_terminated(&self) -> bool {
        self.inner.state() == State::Closed
            || (self.inner.is_error() && self.tries >= Self::MAX_TRIES)
    }

    /// Takes the current time, and returns a collection of updates to apply to the current
    /// state. Will automatically reconnect and clear state if/when the underlying connection is new.
    ///
    /// TODO: Until further notice, it is the caller's responsibility to apply the state changes.
    pub fn update(&mut self, state: &mut S, time_seconds: f32) -> Vec<I> {
        if self.is_closed() {
            self.was_closed = true;
        } else if self.was_closed && self.is_open() && self.inner.has_updates() {
            self.was_closed = false;
            // Need to clear state, since websocket is *no longer* closed and has new updates.
            state.reset();
        }

        self.reconnect_if_necessary(state, time_seconds);
        self.inner.receive_updates()
    }

    /// Reset the host (for future connections) to a different value.
    pub fn reset_host(&mut self, host: String) {
        self.host = host;
    }

    /// Reset the preamble (for future connections) to a different value.
    pub fn reset_preamble(&mut self, preamble: O) {
        self.preamble = Some(preamble);
    }

    /// Sets the format that will be used to send subsequent messages.
    pub fn set_protocol(&mut self, protocol: WebSocketProtocol) {
        self.inner.set_protocol(protocol);
    }

    /// Sends a message, or queues it for sending when the underlying connection is open.
    pub fn send(&mut self, msg: O) {
        self.inner.send(msg);
    }

    /// Attempts to reestablish a connection if necessary. This does not and should not preserve
    /// pending messages.
    fn reconnect_if_necessary(&mut self, state: &mut S, time_seconds: f32) {
        if self.inner.state() == State::Open {
            // Reconnected, forget tries.
            self.tries = 0;
            self.next_try = time_seconds + Self::SECONDS_PER_TRY * 0.5;
        } else if time_seconds < self.next_try {
            // Wait...
        } else if self.inner.is_error() && self.tries < Self::MAX_TRIES {
            // Try again.
            self.inner = ProtoWebSocket::new(&self.host, self.inner.protocol());
            if let Some(p) = self.preamble.as_ref() {
                self.inner.send(p.clone());
            }
            self.tries += 1;
            self.next_try = time_seconds + Self::SECONDS_PER_TRY;
        } else if self.is_terminated() {
            // Stop trying, stop giving the impression of working.
            state.reset();
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
