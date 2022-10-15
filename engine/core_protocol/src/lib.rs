// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(const_option)]
#![feature(once_cell)]

use std::time::{SystemTime, UNIX_EPOCH};

mod owned;

pub mod dto;
pub mod id;
pub mod metrics;
pub mod name;
pub mod rpc;
pub mod serde_util;
pub mod web_socket;

pub type UnixTime = u64;

pub fn get_unix_time_now() -> UnixTime {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis() as u64,
        _ => 0,
    }
}
