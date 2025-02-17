// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(hash_raw_entry)]
#![feature(hash_extract_if)]
#![feature(extract_if)]
#![feature(must_not_suspend)]
#![feature(binary_heap_into_iter_sorted)]
#![feature(stmt_expr_attributes)]
#![feature(iter_intersperse)]
#![feature(let_chains)]

use kodiak_client::GameClient;

use crate::game::Mk48Game;

mod animation;
mod armament;
mod audio;
mod background;
mod camera;
mod game;
mod interpolated;
mod interpolated_contact;
mod particle;
mod settings;
mod sortable_sprite;
mod sprite;
mod state;
mod tessellation;
mod trail;
mod ui;
mod weather;

fn main() {
    Mk48Game::run();
}
