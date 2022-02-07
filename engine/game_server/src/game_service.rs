// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::context::{CoreStatus, PlayerTuple};
use actix::Message;
use common_util::ticks::Ticks;
use core_protocol::dto::RulesDto;
use core_protocol::id::{GameId, PlayerId, TeamId};
use core_protocol::rpc::ServerUpdate;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::marker::Send;
use std::sync::Arc;
use std::time::Duration;

/// A modular game service (representing one arena).
pub trait GameArenaService: 'static + Unpin + Sized + Sync {
    const GAME_ID: GameId;
    /// How long a player can remain in limbo after they lose connection.
    const LIMBO: Duration;

    type Bot: 'static + Bot<Self>;
    type ClientData: 'static + Default + Unpin + Send + Sync;
    type ClientUpdate: 'static + Message<Result = ()> + Send + Serialize;
    type Command: 'static + DeserializeOwned + Send + Unpin;
    type PlayerData: 'static + Default + Unpin + Send + Sync + Debug;
    type PlayerExtension: 'static + Default + Unpin + Send + Sync;
    type BotUpdate<'a>
    where
        Self: 'a;

    fn new(min_players: usize) -> Self;

    fn get_rules(&self) -> RulesDto {
        RulesDto::default()
    }

    /// Called when a player joins the game.
    fn player_joined(&mut self, _player_tuple: &Arc<PlayerTuple<Self>>) {}

    /// Called when a player issues a command.
    fn player_command(&mut self, command: Self::Command, player_tuple: &Arc<PlayerTuple<Self>>);

    /// Called when a player's [`TeamId`] changes.
    fn player_changed_team(
        &mut self,
        _player_tuple: &Arc<PlayerTuple<Self>>,
        _old_team: Option<TeamId>,
    ) {
    }

    /// Called when a player leaves the game.
    fn player_left(&mut self, _player_tuple: &Arc<PlayerTuple<Self>>) {}

    // TODO: this leaves the timing of updates to the infrastructure.
    fn get_client_update(
        &self,
        counter: Ticks,
        player: &Arc<PlayerTuple<Self>>,
        player_tuple: &mut Self::ClientData,
    ) -> Option<Self::ClientUpdate>;
    /// If None, bot quits.
    fn get_bot_update<'a>(
        &'a self,
        counter: Ticks,
        player_tuple: &'a Arc<PlayerTuple<Self>>,
    ) -> Self::BotUpdate<'a>;
    fn get_core_status(&self, player_tuple: &Arc<PlayerTuple<Self>>) -> Option<CoreStatus>;
    fn peek_core(&mut self, _inbound: &ServerUpdate) {}
    /// Before sending.
    fn update(&mut self, ticks: Ticks, counter: Ticks);
    /// After sending.
    fn post_update(&mut self) {}
}

pub trait Bot<G: GameArenaService>: Default + Unpin + Sized + Send {
    /// None indicates quitting.
    fn update<'a>(&mut self, update: G::BotUpdate<'a>, player_id: PlayerId) -> Option<G::Command>;
}
