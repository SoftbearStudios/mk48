// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::unused_context::PlayerData;
use actix::Message;
use common_util::unused_ticks::Ticks;
use core_protocol::id::{GameId, PlayerId};
use core_protocol::rpc::ServerUpdate;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::Send;
use std::time::Duration;

/// A modular game service (representing one arena).
pub trait GameArenaService {
    const GAME_ID: GameId;
    /// How long a player can remain in limbo after they lose connection.
    const LIMBO: Duration;

    type Bot: 'static + Default;
    type ClientData: 'static + Default;
    type ClientUpdate: 'static + Message + Send + Serialize + Update<Self>;
    type Command: 'static + DeserializeOwned;
    type PlayerData: 'static + Default;
    type UpdateRef;

    fn init(&mut self);
    fn start(&mut self) {}
    fn stop(&mut self) {}

    // TODO: this leaves the timing of updates to the infrastructure.
    fn get_player_update(
        &self,
        player_id: PlayerId,
        player: &PlayerData<Self>,
    ) -> Option<Self::UpdateRef>;
    fn peek_core(&mut self, inbound: &ServerUpdate);
    fn update(&mut self, ticks: Ticks);
}

pub trait Update<G: GameArenaService> {
    fn from_update_ref(update_ref: G::UpdateRef) -> Self {
        unimplemented!()
    }
}
