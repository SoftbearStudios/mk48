// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod rate_limiter;
pub mod reconn_web_socket;
pub mod web_socket;

use core_protocol::name::Referrer;
use glam::{vec3, vec4, Vec3, Vec4};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
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

pub fn referrer() -> Option<Referrer> {
    Referrer::new(&web_sys::window().unwrap().document().unwrap().referrer())
}

pub fn ws_protocol() -> &'static str {
    if web_sys::window().unwrap().location().protocol().unwrap() == "http:" {
        "ws"
    } else {
        "wss"
    }
}

pub fn rgb(r: u8, b: u8, g: u8) -> Vec3 {
    vec3(r as f32, b as f32, g as f32) * (1.0 / 255.0)
}

pub fn rgba(r: u8, b: u8, g: u8, a: u8) -> Vec4 {
    vec4(r as f32, b as f32, g as f32, a as f32) * (1.0 / 255.0)
}

pub fn gray(v: u8) -> Vec3 {
    Vec3::splat(v as f32 * (1.0 / 255.0))
}

/// Counts frames per second over a period.
pub struct FpsMonitor {
    elapsed: f32,
    period: f32,
    frames: u32,
    last_sample: Option<f32>,
}

impl FpsMonitor {
    pub fn new(period: f32) -> Self {
        Self {
            period,
            elapsed: 0.0,
            frames: 0,
            last_sample: None,
        }
    }

    /// Returns the fps for the previous period.
    pub fn last_sample(&self) -> Option<f32> {
        self.last_sample
    }

    /// Updates the counter. Returns Some if the period has elapsed.
    pub fn update(&mut self, delta_seconds: f32) -> Option<f32> {
        self.frames = self.frames.saturating_add(1);
        self.elapsed += delta_seconds;

        if self.elapsed
            >= if self.last_sample.is_none() {
                self.period * 0.1
            } else {
                self.period
            }
        {
            let fps = self.frames as f32 / self.elapsed as f32;
            self.elapsed = 0.0;
            self.frames = 0;
            self.last_sample = Some(fps);
            Some(fps)
        } else {
            None
        }
    }
}
