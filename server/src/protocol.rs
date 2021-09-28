// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::server::*;
use crate::world::World;
use actix::prelude::*;
use actix::Recipient;
use common::protocol::*;
use core_protocol::id::{PlayerId, SessionId};
use servutil::observer::ObserverUpdate;

pub type Client = Recipient<ObserverUpdate<Update>>;

/// For main to authenticate SessionIds before opening a websocket.
#[derive(Message)]
#[rtype(result = "Option<PlayerId>")]
pub struct Authenticate {
    pub session_id: SessionId,
}

/// All client->server commands use this unified interface.
pub trait CommandTrait {
    fn apply(
        &self,
        world: &mut World,
        shared_data: &mut SharedData,
        bot: bool,
    ) -> Result<(), &'static str>;
}

pub trait AsCommandTrait {
    fn as_command(&self) -> &dyn CommandTrait;
}

impl AsCommandTrait for Command {
    fn as_command(&self) -> &dyn CommandTrait {
        match *self {
            Command::Control(ref v) => v as &dyn CommandTrait,
            Command::Fire(ref v) => v as &dyn CommandTrait,
            Command::Pay(ref v) => v as &dyn CommandTrait,
            Command::Spawn(ref v) => v as &dyn CommandTrait,
            Command::Upgrade(ref v) => v as &dyn CommandTrait,
        }
    }
}
