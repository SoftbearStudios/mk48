// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::player::{PlayerTuple, PlayerTupleRepo};
use crate::server::Server;
use crate::team::TeamRepo;
use crate::world::World;
use common::protocol::*;
use kodiak_server::{InvitationDto, RankNumber};
use std::sync::Arc;

/// All client->server commands use this unified interface.
pub trait CommandTrait {
    fn apply(
        &self,
        world: &mut World,
        player_tuple: &Arc<PlayerTuple>,
        players: &PlayerTupleRepo,
        teams: &mut TeamRepo<Server>,
        invitation_accepted: Option<InvitationDto>,
        rank: Option<RankNumber>,
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
            Command::Team(ref v) => v as &dyn CommandTrait,
        }
    }
}
