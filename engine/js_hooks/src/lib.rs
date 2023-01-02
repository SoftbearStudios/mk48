// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![warn(missing_docs)]
#![crate_name = "js_hooks"]

//! # Js Hooks
//!
//! [`js_hooks`][`crate`] is a collection of utilities for a WASM application in a JavaScript environment.

use js_sys::Reflect;
use std::fmt;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlCanvasElement, Window};

/// Gets the window.
pub fn window() -> Window {
    web_sys::window().expect("no window")
}

/// Gets the document.
pub fn document() -> Document {
    window().document().expect("no document")
}

/// Gets the canvas for use with WebGL.
pub fn canvas() -> HtmlCanvasElement {
    document()
        .get_element_by_id("canvas")
        .expect("no canvas")
        .dyn_into::<HtmlCanvasElement>()
        .expect("invalid canvas")
}

/// Returns if the mouse pointer is locked.
pub fn pointer_locked() -> bool {
    document().pointer_lock_element().is_some()
}

/// Requests [`canvas`] to be pointer locked. Must call during click event.
pub fn request_pointer_lock() {
    canvas().request_pointer_lock()
}

/// Extracts an error message from a JavaScript error.
pub fn error_message(error: &JsValue) -> Option<String> {
    Reflect::get(error, &JsValue::from_str("message"))
        .as_ref()
        .ok()
        .and_then(JsValue::as_string)
}

/// Log an error to JavaScript's console. Use this instead of [`eprintln!`].
#[macro_export]
macro_rules! console_error {
    ($($t:tt)*) => {
        $crate::log_args(&format_args!($($t)*))
    };
}

/// Log to JavaScript's console. Use this instead of [`println!`].
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => {
        $crate::log_args(&format_args!($($t)*))
    };
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

}

#[doc(hidden)]
pub fn error_args(args: &fmt::Arguments) {
    error(&args.to_string())
}

#[doc(hidden)]
pub fn log_args(args: &fmt::Arguments) {
    log(&args.to_string())
}
