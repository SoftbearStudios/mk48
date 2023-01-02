// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::context::Context;
use crate::player::{PlayerRepo, PlayerTuple};
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
    /// The length of a tick in seconds.
    const TICK_PERIOD_SECS: f32;
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

    /// Returning zero would disable teams.
    fn team_members_max(_players_online: usize) -> usize {
        6
    }

    /// Called when a player joins the game.
    fn player_joined(
        &mut self,
        player_tuple: &Arc<PlayerTuple<Self>>,
        _players: &PlayerRepo<Self>,
    ) {
        let _ = player_tuple;
    }

    /// Called when a player issues a command.
    fn player_command(
        &mut self,
        command: Self::GameRequest,
        player_tuple: &Arc<PlayerTuple<Self>>,
        _players: &PlayerRepo<Self>,
    ) -> Option<Self::GameUpdate>;

    /// Called when a player's [`TeamId`] changes.
    fn player_changed_team(
        &mut self,
        player_tuple: &Arc<PlayerTuple<Self>>,
        old_team: Option<TeamId>,
        _players: &PlayerRepo<Self>,
    ) {
        let _ = player_tuple;
        let _ = old_team;
    }

    /// Called when a player leaves the game. Responsible for clearing player data as necessary.
    fn player_left(&mut self, player_tuple: &Arc<PlayerTuple<Self>>, _players: &PlayerRepo<Self>) {
        let _ = player_tuple;
    }

    fn chat_command(
        &mut self,
        command: &str,
        player_id: PlayerId,
        players: &PlayerRepo<Self>,
    ) -> Option<String> {
        let _ = (command, player_id, players);
        None
    }

    /// Gets a client a.k.a. real player's [`GameUpdate`].
    /// Note that mutable borrowing of the player_tuple is not permitted (will panic).
    ///
    /// Expected, but not necessarily required, to be idempotent.
    fn get_game_update(
        &self,
        player_tuple: &Arc<PlayerTuple<Self>>,
        client_data: &mut Self::ClientData,
        _players: &PlayerRepo<Self>,
    ) -> Option<Self::GameUpdate>;

    /// Returns true iff the player is considered to be "alive" i.e. they cannot change their alias.
    fn is_alive(&self, player_tuple: &Arc<PlayerTuple<Self>>) -> bool;
    /// Before sending.
    fn tick(&mut self, context: &mut Context<Self>);
    /// After sending.
    fn post_update(&mut self, context: &mut Context<Self>) {
        let _ = context;
    }
}

/// Implemented by game bots.
pub trait Bot<G: GameArenaService>: Default + Unpin + Sized + Send {
    /// See bot.rs for explanation.
    const DEFAULT_MIN_BOTS: usize = 30;
    /// See bot.rs for explanation.
    const DEFAULT_MAX_BOTS: usize = usize::MAX;
    /// See bot.rs for explanation.
    const DEFAULT_BOT_PERCENT: usize = 80;

    type Input<'a>
    where
        G: 'a;

    /// Note that mutable borrowing of the player_tuple is not permitted (will panic).
    fn get_input<'a>(
        game: &'a G,
        player_tuple: &'a Arc<PlayerTuple<G>>,
        _players: &'a PlayerRepo<G>,
    ) -> Self::Input<'a>;

    /// None indicates quitting.
    fn update<'a>(
        &mut self,
        update: Self::Input<'a>,
        player_id: PlayerId,
        _players: &'a PlayerRepo<G>,
    ) -> BotAction<G::GameRequest>;
}

#[derive(Debug)]
pub enum BotAction<GR> {
    Some(GR),
    None,
    Quit,
}

impl<GR> Default for BotAction<GR> {
    fn default() -> Self {
        Self::None
    }
}

// What follows is testing related code.
#[cfg(test)]
pub struct MockGame;

#[cfg(test)]
#[derive(Default)]
pub struct MockGameBot;

#[cfg(test)]
impl Bot<MockGame> for MockGameBot {
    type Input<'a> = ();

    fn get_input<'a>(
        _game: &'a MockGame,
        _player_tuple: &'a Arc<PlayerTuple<MockGame>>,
        _players: &PlayerRepo<MockGame>,
    ) -> Self::Input<'a> {
        ()
    }

    fn update<'a>(
        &mut self,
        _update: Self::Input<'_>,
        _player_id: PlayerId,
        _players: &PlayerRepo<MockGame>,
    ) -> BotAction<<MockGame as GameArenaService>::GameRequest> {
        BotAction::None
    }
}

#[cfg(test)]
impl GameArenaService for MockGame {
    const GAME_ID: GameId = GameId::Redacted;
    const TICK_PERIOD_SECS: f32 = 0.5;

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

    fn team_members_max(_players_online: usize) -> usize {
        4
    }

    fn player_command(
        &mut self,
        _command: Self::GameRequest,
        _player_tuple: &Arc<PlayerTuple<Self>>,
        _players: &PlayerRepo<Self>,
    ) -> Option<Self::GameUpdate> {
        None
    }

    fn get_game_update(
        &self,
        _player: &Arc<PlayerTuple<Self>>,
        _player_tuple: &mut Self::ClientData,
        _players: &PlayerRepo<Self>,
    ) -> Option<Self::GameUpdate> {
        Some(())
    }

    fn is_alive(&self, _player_tuple: &Arc<PlayerTuple<Self>>) -> bool {
        false
    }

    fn tick(&mut self, _context: &mut Context<Self>) {}
}
