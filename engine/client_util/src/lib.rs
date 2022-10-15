// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(drain_filter)]
#![feature(must_not_suspend)]
#![feature(hash_drain_filter)]
#![feature(once_cell)]
#![feature(variant_count)]
#![feature(result_into_ok_or_err)]
#![feature(result_option_inspect)]

extern crate core;

pub mod apply;
#[cfg(feature = "audio")]
pub mod audio;
pub mod browser_storage;
pub mod context;
pub mod fps_monitor;
pub mod frontend;
pub mod game_client;
pub mod infrastructure;
#[cfg(feature = "joined")]
pub mod joined;
pub mod joystick;
pub mod js_util;
pub mod keyboard;
pub mod mouse;
pub mod rate_limiter;
pub mod reconn_web_socket;
pub mod setting;
pub mod visibility;
pub mod web_socket;
