// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::context::Context;
use crate::player::PlayerTuple;
use common_util::ticks::Ticks;
use core_protocol::id::{GameId, PlayerId, TeamId};
use core_protocol::name::PlayerAlias;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::marker::Send;
use std::sync::Arc;
use std::time::Duration;

/// A modular game service (representing one arena).
pub trait GameArenaService: 'static + Unpin + Sized + Send + Sync {
    const GAME_ID: GameId;
    /// How long a player can remain in limbo after they lose connection.
    const LIMBO: Duration = Duration::from_secs(6);
    /// Start player score at this.
    const DEFAULT_SCORE: u32 = 0;
    /// Minimum score to report another player, to slow report-abuse.
    const MINIMUM_REPORT_SCORE: u32 = 100;
    /// How many players to display on the leaderboard (and liveboard).
    const LEADERBOARD_SIZE: usize = 10;
    /// Whether to display bots on liveboard. Bots are never saved to the leaderboard.
    const LIVEBOARD_BOTS: bool = false;
    /// Leaderboard won't be touched if player count is below.
    const LEADERBOARD_MIN_PLAYERS: usize = 10;
    /// Maximum number of players in a team before no more can be accepted.
    /// Set to zero to disable teams.
    const TEAM_MEMBERS_MAX: usize = 6;
    /// Maximum number of players trying to join a team at once.
    const TEAM_JOINERS_MAX: usize = 6;
    /// Maximum number of teams a player may try to join at once, before old requests are cancelled.
    const TEAM_JOINS_MAX: usize = 3;

    type Bot: 'static + Bot<Self>;
    type ClientData: 'static + Default + Debug + Unpin + Send + Sync;
    type GameUpdate: 'static + Sync + Send + Serialize;
    type GameRequest: 'static + DeserializeOwned + Send + Unpin;
    type PlayerData: 'static + Default + Unpin + Send + Sync + Debug;
    type PlayerExtension: 'static + Default + Unpin + Send + Sync;

    fn new(min_players: usize) -> Self;

    /// Get alias of authority figure (that, for example, sends chat moderation warnings).
    fn authority_alias() -> PlayerAlias {
        PlayerAlias::new_unsanitized("Server")
    }

    /// Generate a default player alias. It may be the same or different (e.g. random) each time.
    fn default_alias() -> PlayerAlias {
        PlayerAlias::new_unsanitized("Guest")
    }

    /// Called when a player joins the game.
    fn player_joined(&mut self, _player_tuple: &Arc<PlayerTuple<Self>>) {}

    /// Called when a player issues a command.
    fn player_command(
        &mut self,
        command: Self::GameRequest,
        player_tuple: &Arc<PlayerTuple<Self>>,
    ) -> Option<Self::GameUpdate>;

    /// Called when a player's [`TeamId`] changes.
    fn player_changed_team(
        &mut self,
        _player_tuple: &Arc<PlayerTuple<Self>>,
        _old_team: Option<TeamId>,
    ) {
    }

    /// Called when a player leaves the game. Responsible for clearing player data as necessary.
    fn player_left(&mut self, _player_tuple: &Arc<PlayerTuple<Self>>) {}

    /// Gets a client a.k.a. real player's [`GameUpdate`].
    /// Note that mutable borrowing of the player_tuple is not permitted (will panic).
    ///
    /// Expected, but not necessarily required, to be idempotent.
    fn get_game_update(
        &self,
        counter: Ticks,
        player_tuple: &Arc<PlayerTuple<Self>>,
        client_data: &mut Self::ClientData,
    ) -> Option<Self::GameUpdate>;

    /// Returns true iff the player is considered to be "alive" i.e. they cannot change their alias.
    fn is_alive(&self, player_tuple: &Arc<PlayerTuple<Self>>) -> bool;
    /// Before sending.
    fn tick(&mut self, context: &Context<Self>);
    /// After sending.
    fn post_update(&mut self, _context: &Context<Self>) {}
}

/// Implemented by game bots.
pub trait Bot<G: GameArenaService>: Default + Unpin + Sized + Send {
    type Input<'a>
    where
        G: 'a;

    /// Note that mutable borrowing of the player_tuple is not permitted (will panic).
    fn get_input<'a>(
        game: &'a G,
        counter: Ticks,
        player_tuple: &'a Arc<PlayerTuple<G>>,
    ) -> Self::Input<'a>;

    /// None indicates quitting.
    fn update<'a>(
        &mut self,
        update: Self::Input<'a>,
        player_id: PlayerId,
    ) -> Option<G::GameRequest>;
}

// What follows is testing related code.
#[cfg(test)]
pub struct MockGame;

#[cfg(test)]
#[derive(Default)]
pub struct MockGameBot;

#[cfg(test)]
impl Bot<MockGame> for MockGameBot {
    type BotUpdate<'a> = ();

    fn get_input<'a>(
        game: &'a MockGame,
        _counter: Ticks,
        _player_tuple: &'a Arc<PlayerTuple<Self>>,
    ) -> Self::BotUpdate<'a> {
        ()
    }

    fn update<'a>(
        &mut self,
        _update: <MockGame as GameArenaService>::BotUpdate<'_>,
        _player_id: PlayerId,
    ) -> Option<<MockGame as GameArenaService>::GameRequest> {
        Some(())
    }
}

#[cfg(test)]
impl GameArenaService for MockGame {
    const GAME_ID: GameId = GameId::Redacted;

    const TEAM_MEMBERS_MAX: usize = 4;
    const TEAM_JOINERS_MAX: usize = 3;
    const TEAM_JOINS_MAX: usize = 2;

    type Bot = MockGameBot;
    type ClientData = ();
    type GameUpdate = ();
    type GameRequest = ();
    type PlayerData = ();
    type PlayerExtension = ();

    fn new(_min_players: usize) -> Self {
        Self
    }

    fn player_command(
        &mut self,
        _command: Self::GameRequest,
        _player_tuple: &Arc<PlayerTuple<Self>>,
    ) -> Option<Self::GameUpdate> {
        None
    }

    fn get_game_update(
        &self,
        _counter: Ticks,
        _player: &Arc<PlayerTuple<Self>>,
        _player_tuple: &mut Self::ClientData,
    ) -> Option<Self::GameUpdate> {
        Some(())
    }

    fn is_alive(&self, _player_tuple: &Arc<PlayerTuple<Self>>) -> bool {
        false
    }

    fn tick(&mut self, _context: &Context<Self>) {}
}
