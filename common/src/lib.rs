// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(array_chunks)]
#![feature(test)]

use kodiak_common::{DefaultedGameConstants, GameConstants};

// Actually required see https://doc.rust-lang.org/beta/unstable-book/library-features/test.html
#[cfg(test)]
extern crate core;
#[cfg(test)]
extern crate test;

pub const MK48_CONSTANTS: &'static GameConstants = &GameConstants {
    game_id: "Mk48",
    name: "Mk48.io",
    domain: "mk48.io",
    geodns_enabled: true,
    trademark: "Mk48.io",
    server_names: &[
        "Atlantic", "Pacific", "Fjord", "Kraken", "Scotia", "Barents", "Bering", "Chukchi",
    ],
    defaulted: DefaultedGameConstants::new(),
};

pub mod altitude;
pub mod angle;
pub mod complete;
pub mod contact;
pub mod death_reason;
pub mod entity;
pub mod guidance;
pub mod protocol;
pub mod terrain;
pub mod ticks;
pub mod transform;
pub mod util;
pub mod velocity;
pub mod world;
