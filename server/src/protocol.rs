// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::server::*;
use crate::world::World;
use common::protocol::*;
use game_server::player::PlayerTuple;
use std::sync::Arc;

/// All client->server commands use this unified interface.
pub trait CommandTrait {
    fn apply(
        &self,
        world: &mut World,
        player_tuple: &Arc<PlayerTuple<Server>>,
    ) -> Result<(), &'static str>;
}

pub trait AsCommandTrait {
    fn as_command(&self) -> &dyn CommandTrait;
}

impl AsCommandTrait for Command {
    fn as_command(&self) -> &dyn CommandTrait {
        match *self {
            Command::Control(ref v) => v as &dyn CommandTrait,
            Command::Spawn(ref v) => v as &dyn CommandTrait,
            Command::Upgrade(ref v) => v as &dyn CommandTrait,
        }
    }
}
