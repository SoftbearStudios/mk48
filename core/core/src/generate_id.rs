// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::get_unix_time_now;
use rand::Rng;
use std::num::{NonZeroU32, NonZeroU64};

const DAY_BITS: u32 = 10;

/// Generates a random 32 bit id.
/// To check if unique, only need to check against items created in the last 24 hours (and items must not
/// be able to live more than 2.8 years).
pub fn generate_id() -> NonZeroU32 {
    let unix_millis = get_unix_time_now();
    let unix_days = (unix_millis / (24 * 60 * 60 * 1000)) as u32;
    let mut r: u32 = rand::thread_rng().gen();
    if r == 0 {
        // Preserve non-zero guarantee.
        r = 1;
    }
    // Top 10 bits are from day, bottom.
    NonZeroU32::new(unix_days.wrapping_shl(32 - DAY_BITS) | (r & ((1 << (32 - DAY_BITS)) - 1)))
        .unwrap()
}

/// Generates a random 64 bit id.
/// See `generate_id` for more info.
pub fn generate_id_64() -> NonZeroU64 {
    let unix_millis = get_unix_time_now();
    let unix_days = (unix_millis / (24 * 60 * 60 * 1000)) as u64;
    let mut r: u64 = rand::thread_rng().gen();
    if r == 0 {
        // Preserve non-zero guarantee.
        r = 1;
    }
    // Top 10 bits are from day, bottom.
    NonZeroU64::new(unix_days.wrapping_shl(64 - DAY_BITS) | (r & ((1 << (64 - DAY_BITS)) - 1)))
        .unwrap()
}
