// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(array_chunks)]
#![feature(box_syntax)]
#![feature(mixed_integer_ops)]
#![feature(result_into_ok_or_err)]
#![feature(test)]

// Actually required see https://doc.rust-lang.org/beta/unstable-book/library-features/test.html
#[cfg(test)]
extern crate core;
#[cfg(test)]
extern crate test;

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
