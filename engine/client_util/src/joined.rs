// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::str::FromStr;
use std::sync::LazyLock;
use web_sys::{window, Storage};

const KEY: &'static str = "joined";

fn local_storage() -> Option<Storage> {
    window().unwrap().local_storage().ok().flatten()
}

/// Unix seconds.
fn now_seconds() -> f64 {
    js_sys::Date::now() * (1.0 / 1000.0)
}

pub(crate) fn init() {
    if let Some(local_storage) = local_storage() {
        if local_storage.get_item(KEY).ok().flatten().is_none() {
            let _ = local_storage.set_item(KEY, &now_seconds().to_string());
        }
    }
}

/// Unix timestamp seconds.
pub fn timestamp_seconds() -> f64 {
    static CACHE: LazyLock<f64> = LazyLock::new(|| {
        local_storage()
            .and_then(|local_storage| local_storage.get_item(KEY).ok().flatten())
            .and_then(|s| {
                f64::from_str(&s)
                    .ok()
                    .filter(|f| f.is_finite() && *f >= now_seconds())
            })
            .unwrap_or(0.0)
    });
    *CACHE
}

/// Minutes since joined game, rounded down.
pub fn minutes_since_u8() -> u8 {
    ((now_seconds() - timestamp_seconds()) * (1.0 / 60.0)) as u8
}
