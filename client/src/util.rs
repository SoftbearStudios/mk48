// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{Vec3, Vec4};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (crate::util::log(&format_args!($($t)*).to_string()))
}

/// e.g. foo.mk48.io
pub fn host() -> String {
    web_sys::window().unwrap().location().host().unwrap()
}

/// e.g. mk48.io
pub fn domain_name() -> String {
    let h = host();
    let mut split: Vec<_> = h.split('.').collect();
    if split.len() > 2 {
        let tld = split.pop().unwrap();
        let domain = split.pop().unwrap();
        domain.to_owned() + "." + tld
    } else {
        h
    }
}

pub fn ws_protocol() -> &'static str {
    if web_sys::window().unwrap().location().protocol().unwrap() == "http:" {
        "ws"
    } else {
        "wss"
    }
}

pub fn rgb(r: u8, b: u8, g: u8) -> Vec3 {
    Vec3::new(r as f32, b as f32, g as f32) * (1.0 / 255.0)
}

pub fn rgba(r: u8, b: u8, g: u8, a: u8) -> Vec4 {
    Vec4::new(r as f32, b as f32, g as f32, a as f32) * (1.0 / 255.0)
}

pub fn gray(v: u8) -> Vec3 {
    Vec3::splat(v as f32 * (1.0 / 255.0))
}
