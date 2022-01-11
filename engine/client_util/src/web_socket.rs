// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::console_log;
use core_protocol::web_socket::WebSocketFormat;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::RefCell;
use std::mem;
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
    format: WebSocketFormat,
    state: State,
    outbound_buffer: Vec<O>,
    inbound_buffer: Vec<I>, // Only used in State::Opening.
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
    pub fn new(host: &str, format: WebSocketFormat) -> Self {
        let ret = Self {
            inner: Rc::new(RefCell::new(ProtoWebSocketInner {
                socket: WebSocket::new(&format!("{}?format={}", host, format.as_str())).unwrap(),
                inbound_buffer: Vec::new(),
                outbound_buffer: Vec::new(),
                format,
                state: State::Opening,
            })),
        };

        let local_inner_rc = ret.inner.clone();
        let local_inner = local_inner_rc.deref().borrow_mut();

        let inner_copy = ret.inner.clone();

        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            // Handle difference Text/Binary,...
            let update = if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                //console_log!("message event, received arraybuffer: {:?}", abuf);
                let buf = js_sys::Uint8Array::new(&array_buffer).to_vec();
                bincode::deserialize(&buf).unwrap()
            } else if let Ok(t) = e.data().dyn_into::<js_sys::JsString>() {
                //console_log!("message event, received Text: {:?}", txt);
                let text: String = t.into();
                serde_json::from_str::<I>(&text).unwrap()
            } else {
                console_log!("message event, received Unknown: {:?}", e.data());
                return;
            };

            inner_copy.deref().borrow_mut().inbound_buffer.push(update);
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
            let mut outbounds = Vec::new();
            mem::swap(&mut outbounds, &mut inner.outbound_buffer);
            for outbound in outbounds {
                Self::do_send(&inner.socket, outbound, inner.format);
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
        let mut inbounds = Vec::new();
        mem::swap(&mut inbounds, &mut inner.inbound_buffer);
        inbounds
    }

    /// Send a message or buffer it if the websocket is still opening.
    pub fn send(&mut self, msg: O) {
        let mut inner = self.inner.deref().borrow_mut();
        match inner.state {
            State::Opening => inner.outbound_buffer.push(msg),
            State::Open => Self::do_send(&inner.socket, msg, inner.format),
            _ => console_log!("cannot send on closed websocket."),
        }
    }

    /// Sends a message or drop it on error.
    fn do_send(socket: &WebSocket, msg: O, format: WebSocketFormat) {
        match format {
            WebSocketFormat::Binary => {
                let buf = bincode::serialize(&msg).unwrap();
                if socket.send_with_u8_array(&buf).is_err() {
                    console_log!("error sending binary on ws");
                }
            }
            WebSocketFormat::Json => {
                let buf = serde_json::to_string(&msg).unwrap();
                if socket.send_with_str(&buf).is_err() {
                    console_log!("error sending text on ws");
                }
            }
        }
    }
}

impl<I, O> ProtoWebSocket<I, O> {
    pub fn format(&mut self) -> WebSocketFormat {
        self.inner.borrow().format
    }

    pub fn set_format(&mut self, format: WebSocketFormat) {
        self.inner.borrow_mut().format = format;
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
            _ => console_log!("cannot close closed websocket."),
        }
    }
}
