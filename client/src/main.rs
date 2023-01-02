// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(hash_raw_entry)]
#![feature(hash_drain_filter)]
#![feature(drain_filter)]
#![feature(must_not_suspend)]
#![feature(binary_heap_into_iter_sorted)]
#![feature(stmt_expr_attributes)]
#![feature(option_result_contains)]
#![feature(mixed_integer_ops)]
#![feature(iter_intersperse)]

use crate::game::Mk48Game;
use crate::ui::{Mk48Route, Mk48Ui};

mod animation;
mod armament;
mod audio;
mod background;
mod camera;
mod game;
mod interpolated;
mod interpolated_contact;
mod licenses;
mod particle;
mod settings;
mod sortable_sprite;
mod sprite;
mod state;
mod tessellation;
mod trail;
mod translation;
mod ui;
mod weather;

fn main() {
    yew_frontend::entry_point::<Mk48Game, Mk48Ui, Mk48Route>();
}
