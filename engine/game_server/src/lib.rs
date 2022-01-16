// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(drain_filter)]
#![feature(new_uninit)]
#![feature(get_mut_unchecked)]
#![feature(async_closure)]
#![feature(hash_drain_filter)]
#![feature(generic_associated_types)]

mod bot;
pub mod context;
pub mod entry_point;
pub mod game_service;
pub mod infrastructure;
pub mod protocol;
