// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(hash_raw_entry)]
#![feature(hash_drain_filter)]
#![feature(drain_filter)]
#![feature(must_not_suspend)]
#![feature(bool_to_option)]

mod animation;
mod game;
mod interpolated_contact;
mod settings;
mod ui;
mod zoom;

client_util::entry_point!(crate::game::Mk48Game);
