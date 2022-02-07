// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(exit_status_error)]

pub mod audio;
pub mod texture;

fn shorten_name(name: &str) -> &str {
    let idx = name.rfind('.').unwrap();
    &name[..idx]
}
