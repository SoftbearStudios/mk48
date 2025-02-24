// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(extract_if)]
#![feature(new_uninit)]
#![feature(get_mut_unchecked)]
#![feature(async_closure)]
#![feature(hash_extract_if)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

//! The game server has authority over all game logic. Clients are served the client, which connects
//! via websocket.

mod arena;
mod bot;
mod collision;
mod complete_ref;
mod contact_ref;
mod entities;
mod entity;
mod entity_extension;
mod noise;
mod ordered_set;
mod player;
mod protocol;
mod server;
mod team;
mod terrain_pool;
mod world;
mod world_inbound;
mod world_mutation;
mod world_outbound;
mod world_physics;
mod world_physics_radius;
mod world_spawn;
#[cfg(test)]
mod world_test;

use crate::server::Server;
use kodiak_server::{entry_point, minicdn};
use std::process::ExitCode;

fn main() -> ExitCode {
    noise::init();

    entry_point::<Server>(minicdn::release_include_mini_cdn!("../../client/dist/"))
}
