// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use gloo_events::{EventListener, EventListenerOptions};
use std::marker::PhantomData;
use wasm_bindgen::JsCast;
use web_sys::{window, EventTarget};

/// Listens to a certain event type on the window.
pub struct WindowEventListener<E> {
    _inner: EventListener,
    _spooky: PhantomData<E>,
}

impl<E: JsCast> WindowEventListener<E> {
    fn new_inner(
        target: &EventTarget,
        name: &'static str,
        mut callback: impl FnMut(&E) + 'static,
        allow_prevent_default: bool,
    ) -> Self {
        let options = if allow_prevent_default {
            EventListenerOptions::enable_prevent_default()
        } else {
            EventListenerOptions::default()
        };

        Self {
            _inner: EventListener::new_with_options(target, name, options, move |event| {
                // We use unchecked_ref because browsers can't be bothered to throw FocusEvent
                // and ResizeEvent, and instead throw Event.
                let typed_event = event.unchecked_ref::<E>();
                callback(&typed_event);
            }),
            _spooky: PhantomData,
        }
    }

    pub fn new(
        name: &'static str,
        callback: impl FnMut(&E) + 'static,
        allow_prevent_default: bool,
    ) -> Self {
        Self::new_inner(&window().unwrap(), name, callback, allow_prevent_default)
    }

    /// Goes on the body instead of the window.
    pub fn new_body(
        name: &'static str,
        callback: impl FnMut(&E) + 'static,
        allow_prevent_default: bool,
    ) -> Self {
        Self::new_inner(
            &window().unwrap().document().unwrap().body().unwrap(),
            name,
            callback,
            allow_prevent_default,
        )
    }
}
