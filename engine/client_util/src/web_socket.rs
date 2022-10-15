// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::web_socket::WebSocketProtocol;
use js_hooks::console_error;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

/// The state of a web socket.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum State {
    Opening,
    Open,
    Error,
    Closed,
}

struct ProtoWebSocketInner<I, O> {
    socket: WebSocket,
    protocol: WebSocketProtocol,
    state: State,
    outbound_buffer: Vec<O>,
    /// Only used in State::Opening.
    inbound_buffer: Vec<I>,
}

/// Websocket that obeys a protocol consisting of an inbound and outbound message.
pub struct ProtoWebSocket<I, O> {
    inner: Rc<RefCell<ProtoWebSocketInner<I, O>>>,
}

impl<I, O> ProtoWebSocket<I, O>
where
    I: 'static + DeserializeOwned,
    O: 'static + Serialize,
{
    /// Opens a new websocket.
    pub fn new(host: &str, protocol: WebSocketProtocol) -> Self {
        let ret = Self {
            inner: Rc::new(RefCell::new(ProtoWebSocketInner {
                socket: WebSocket::new(host).unwrap(),
                inbound_buffer: Vec::new(),
                outbound_buffer: Vec::new(),
                protocol,
                state: State::Opening,
            })),
        };

        let local_inner_rc = ret.inner.clone();
        let local_inner = local_inner_rc.deref().borrow_mut();

        let inner_copy = ret.inner.clone();

        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            // Handle difference Text/Binary,...
            let result = if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                //console_log!("message event, received arraybuffer: {:?}", abuf);
                let buf = js_sys::Uint8Array::new(&array_buffer).to_vec();
                bincode::deserialize(&buf).map_err(|e| e.to_string())
            } else if let Ok(_t) = e.data().dyn_into::<js_sys::JsString>() {
                #[cfg(feature = "json")]
                {
                    let t = _t;
                    //console_log!("message event, received Text: {:?}", txt);

                    let text: String = t.into();
                    serde_json::from_str::<I>(&text).map_err(|e| e.to_string())
                }
                #[cfg(not(feature = "json"))]
                {
                    console_error!("message event, json not supported");
                    return;
                }
            } else {
                console_error!("message event, received Unknown: {:?}", e.data());
                return;
            };

            let mut inner = inner_copy.deref().borrow_mut();
            match result {
                Ok(update) => inner.inbound_buffer.push(update),
                Err(e) => {
                    console_error!("error decoding websocket data: {}", e);
                    // Mark as closed without actually closing. This may keep a player's session
                    // alive for longer, so they can save their progress by refreshing. The
                    // refresh menu should encourage this.
                    inner.state = State::Closed;
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        // set message event handler on WebSocket
        local_inner
            .socket
            .set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();

        let inner_copy = ret.inner.clone();
        let onerror_callback = Closure::wrap(Box::new(move |_e: ErrorEvent| {
            // This will be followed by a close even, which is reported to the caller by
            // handle_close
            inner_copy.deref().borrow_mut().state = State::Error;
        }) as Box<dyn FnMut(ErrorEvent)>);
        local_inner
            .socket
            .set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        let inner_copy = ret.inner.clone();
        let onopen_callback = Closure::once(move || {
            let mut inner = inner_copy.deref().borrow_mut();
            inner.state = State::Open;
            for outbound in std::mem::take(&mut inner.outbound_buffer) {
                Self::do_send(&inner.socket, outbound, inner.protocol);
            }
        });
        local_inner
            .socket
            .set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        let inner_copy = ret.inner.clone();
        let onclose_callback = Closure::once(move |e: CloseEvent| {
            let state = &mut inner_copy.deref().borrow_mut().state;
            if e.code() == 1000 {
                // Normal closure.
                if *state != State::Error {
                    *state = State::Closed;
                }
            } else {
                // Abnormal closure.
                *state = State::Error;
            }
        });
        local_inner
            .socket
            .set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();

        local_inner
            .socket
            .set_binary_type(web_sys::BinaryType::Arraybuffer);

        ret
    }

    /// Gets current (cached) websocket state.
    pub fn state(&self) -> State {
        self.inner.borrow().state
    }

    /// Returns whether closed for any reason (error or not).
    pub fn is_closed(&self) -> bool {
        matches!(self.state(), State::Closed | State::Error)
    }

    /// Returns whether closed in error.
    pub fn is_error(&self) -> bool {
        matches!(self.state(), State::Error)
    }

    /// Returns whether socket is open.
    pub fn is_open(&self) -> bool {
        matches!(self.state(), State::Open)
    }

    /// Returns whether `receive_updates` would return a non-empty `Vec`.
    pub fn has_updates(&self) -> bool {
        !self.inner.borrow().inbound_buffer.is_empty()
    }

    /// Gets buffered updates.
    pub fn receive_updates(&mut self) -> Vec<I> {
        let mut inner = self.inner.deref().borrow_mut();
        std::mem::take(&mut inner.inbound_buffer)
    }

    /// Send a message or buffer it if the websocket is still opening.
    pub fn send(&mut self, msg: O) {
        let mut inner = self.inner.deref().borrow_mut();
        match inner.state {
            State::Opening => inner.outbound_buffer.push(msg),
            State::Open => Self::do_send(&inner.socket, msg, inner.protocol),
            _ => console_error!("cannot send on closed websocket."),
        }
    }

    /// Sends a message or drop it on error.
    fn do_send(socket: &WebSocket, msg: O, protocol: WebSocketProtocol) {
        match protocol {
            WebSocketProtocol::Binary => {
                let buf = bincode::serialize(&msg).unwrap();
                if socket.send_with_u8_array(&buf).is_err() {
                    console_error!("error sending binary on ws");
                }
            }
            #[cfg(feature = "json")]
            WebSocketProtocol::Json => {
                let buf = serde_json::to_string(&msg).unwrap();
                if socket.send_with_str(&buf).is_err() {
                    console_error!("error sending text on ws");
                }
            }
        }
    }
}

impl<I, O> ProtoWebSocket<I, O> {
    pub fn protocol(&mut self) -> WebSocketProtocol {
        self.inner.borrow().protocol
    }

    pub fn set_protocol(&mut self, protocol: WebSocketProtocol) {
        self.inner.borrow_mut().protocol = protocol;
    }

    pub fn close(&mut self) {
        let inner = self.inner.deref().borrow();
        match inner.state {
            State::Opening | State::Open => {
                // Calling close may synchronously invoke onerror, which borrows inner. Must drop
                // our borrow first.
                let clone = inner.socket.clone();
                drop(inner);
                let _ = clone.close();
            }
            _ => console_error!("cannot close closed websocket."),
        }
    }
}
