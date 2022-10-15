// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(drain_filter)]
#![feature(new_uninit)]
#![feature(get_mut_unchecked)]
#![feature(async_closure)]
#![feature(hash_drain_filter)]
#![feature(generic_associated_types)]
#![feature(binary_heap_into_iter_sorted)]
#![feature(once_cell)]
#![feature(int_roundings)]
#![feature(is_sorted)]
#![feature(variant_count)]
#![feature(result_into_ok_or_err)]
#![feature(result_option_inspect)]

pub mod admin;
pub mod bot;
pub mod chat;
pub mod client;
pub mod context;
pub mod context_service;
pub mod entry_point;
pub mod game_service;
pub mod infrastructure;
pub mod invitation;
pub mod leaderboard;
pub mod liveboard;
pub mod metric;
pub mod ordered_set;
pub mod player;
pub mod status;
pub mod team;
#[macro_use]
pub mod util;
pub mod discord;
pub(crate) mod log;
pub(crate) mod options;
pub mod static_files;
pub mod system;
