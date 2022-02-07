// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(derive_default_enum)]
#![feature(drain_filter)]
#![feature(must_not_suspend)]
#![feature(hash_raw_entry)]
#![feature(hash_drain_filter)]
#![feature(array_zip)]
#![feature(bool_to_option)]
#![feature(label_break_value)]
#![feature(mixed_integer_ops)]

pub mod apply;
pub mod audio;
pub mod context;
pub mod entry_point;
pub mod fps_monitor;
pub mod game_client;
pub mod infrastructure;
pub mod joystick;
pub mod js_hooks;
pub mod keyboard;
pub mod local_storage;
pub mod mouse;
pub mod rate_limiter;
pub mod reconn_web_socket;
pub mod renderer;
pub mod rgb;
pub mod setting;
pub mod web_socket;

/// Log to javascript console. Use this instead of println!()
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => {
        {
            use wasm_bindgen::prelude::*;

            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = console)]
                pub fn log(s: &str);
            }

            log(&format_args!($($t)*).to_string());
        }
    }
}
