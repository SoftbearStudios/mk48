// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::client::PlayerClientData;
use crate::game_service::GameArenaService;
use crate::invitation::InvitationRepo;
use crate::metric::MetricRepo;
use crate::team::{PlayerTeamData, TeamRepo};
use crate::util::diff_large_n;
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use core_protocol::dto::{InvitationDto, PlayerDto};
use core_protocol::id::{PlayerId, TeamId};
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{PlayerRequest, PlayerUpdate};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Players of an arena.
pub struct PlayerRepo<G: GameArenaService> {
    /// Ground-truth player data. Care must be exercised to avoid mutably borrowing the same player
    /// twice, which will induce a runtime error.
    pub(crate) players: HashMap<PlayerId, Arc<PlayerTuple<G>>>,
    /// Previous DTO's sent to clients.
    previous: Arc<[PlayerDto]>,
    /// Recently computed cache of number of real players (not bots).
    pub(crate) real_players: usize,
    /// Recently computed cache of number of real players (not bots) that were alive recently.
    pub(crate) real_players_live: usize,
}

impl<G: GameArenaService> PlayerRepo<G> {
    pub fn new() -> Self {
        // For testing performance impact of stale clients.
        /*
        use server_util::generate_id::{generate_id, generate_id_64};
        use core_protocol::id::{SessionId};
        use crate::metric::{ClientMetricData};
        use crate::client::{Authenticate};

        let mut players: HashMap<PlayerId, Arc<PlayerTuple<G>>> = HashMap::new();

        for _ in 0..100000 {
            let pid = PlayerId(generate_id());
            if !players.contains_key(&pid) {
                players.insert(
                    pid,
                    Arc::new(PlayerTuple::new(PlayerData::new(
                        pid,
                        Some(Box::new(PlayerClientData::new(
                            SessionId(generate_id_64()),
                            ClientMetricData::from(&Authenticate {
                                ip_address: None,
                                user_agent_id: None,
                                referrer: None,
                                arena_id_session_id: None,
                                invitation_id: None,
                            }),
                            None,
                        ))),
                    ))),
                );
                players.get(&pid).as_ref().unwrap().borrow_player_mut().client_mut().unwrap().status = crate::client::ClientStatus::Stale{expiry: Instant::now() + Duration::from_secs(3600)};
            }
        }
         */

        Self {
            players: HashMap::new(),
            real_players: 0,
            real_players_live: 0,
            previous: Vec::new().into(),
        }
    }

    /// Returns total number of players (including bots).
    pub fn len(&self) -> usize {
        self.players.len()
    }

    /// Tests if the player exists (in cache).
    pub fn contains(&self, player_id: PlayerId) -> bool {
        self.players.contains_key(&player_id)
    }

    /// Gets the player tuple of a given player.
    pub fn get(&self, player_id: PlayerId) -> Option<&Arc<PlayerTuple<G>>> {
        self.players.get(&player_id)
    }

    /// Inserts a player (it is not mandatory to insert this way).
    pub(crate) fn insert(&mut self, player_id: PlayerId, player: Arc<PlayerTuple<G>>) {
        #[cfg(debug_assertions)]
        {
            if let Some(existing) = self.players.get(&player_id) {
                assert_eq!(existing.borrow_player().player_id, player_id);
            }
        }
        self.players.insert(player_id, player);
    }

    /// Removes a player, performing mandatory cleanup steps.
    pub(crate) fn forget(
        &mut self,
        player_id: PlayerId,
        teams: &mut TeamRepo<G>,
        invitations: &mut InvitationRepo<G>,
    ) {
        if let Some(mut player) = self.borrow_player_mut(player_id) {
            invitations.forget_player_invitation(&mut player);
        } else {
            debug_assert!(false, "forgetting non-existent player");
        }
        teams.cleanup_player(player_id, self);

        self.players.remove(&player_id);
    }

    fn report_player(
        &mut self,
        req_player_id: PlayerId,
        report_player_id: PlayerId,
        metrics: &mut MetricRepo<G>,
    ) -> Result<PlayerUpdate, &'static str> {
        if req_player_id == report_player_id {
            return Err("cannot report self");
        }
        let mut req_player = self
            .borrow_player_mut(req_player_id)
            .ok_or("nonexistent player")?;
        if req_player.score < G::MINIMUM_REPORT_SCORE {
            return Err("report requirements unmet");
        }
        let req_client = req_player
            .client_mut()
            .ok_or("only clients can report players")?;
        let mut report_player = self
            .borrow_player_mut(report_player_id)
            .ok_or("cannot report nonexistent player")?;
        let report_client = report_player
            .client_mut()
            .ok_or("only clients can be reported")?;
        if req_client.reported.insert(report_player_id) {
            report_client.chat.context.report();
            metrics.mutate_with(|m| m.abuse_reports.increment(), &report_client.metrics);
            Ok(PlayerUpdate::Reported(report_player_id))
        } else {
            Err("already reported")
        }
    }

    /// Handles an arbitrary [`PlayerRequest`].
    pub(crate) fn handle_player_request(
        &mut self,
        req_player_id: PlayerId,
        request: PlayerRequest,
        metrics: &mut MetricRepo<G>,
    ) -> Result<PlayerUpdate, &'static str> {
        match request {
            PlayerRequest::Report(player_id) => {
                self.report_player(req_player_id, player_id, metrics)
            }
        }
    }

    /// Updates cache of whether players are alive, tallying metrics in the process.
    pub(crate) fn update_is_alive_and_team_id(
        &self,
        service: &mut G,
        teams: &mut TeamRepo<G>,
        metrics: &mut MetricRepo<G>,
    ) {
        for pt in self.iter() {
            let is_alive = service.is_alive(pt);
            let mut p = pt.borrow_player_mut();
            let player_id = p.player_id;

            if is_alive != p.was_alive {
                if is_alive {
                    // Play started.
                    metrics.start_play(&mut p);
                } else {
                    // Play stopped.
                    metrics.stop_play(&mut *p);
                }

                p.was_alive = is_alive;
                p.was_ever_alive = true;
                p.was_alive_timestamp = Instant::now();
            }
            let is_out_of_game = p.is_out_of_game();
            let was_out_of_game = p.was_out_of_game;
            p.was_out_of_game = is_out_of_game;

            drop(p);

            if is_out_of_game && !was_out_of_game {
                teams.cleanup_player(player_id, self);
            }

            p = pt.borrow_player_mut();

            let current_team_id = p.team_id();
            let previous_team_id = p.team.previous_team_id;
            // We will inform the game service later in this function call.
            p.team.previous_team_id = current_team_id;

            drop(p);

            if current_team_id != previous_team_id {
                service.player_changed_team(pt, previous_team_id, self);
            }
        }
    }

    /// Computes current set of player dtos, and number of real players (total and live).
    fn compute(&self, teams: &TeamRepo<G>) -> (Vec<PlayerDto>, usize, usize) {
        let mut real_players = 0;
        let mut real_players_live = 0;

        let player_dtos = self
            .iter_borrow()
            .filter_map(|p| {
                if !p.is_bot() {
                    real_players += 1;
                }

                if p.is_bot() || p.is_out_of_game() {
                    // TODO: Game can optionally allow bots to participate in teams.
                    None
                } else {
                    real_players_live += 1;

                    Some(PlayerDto {
                        alias: p.alias(),
                        moderator: p.client().map(|c| c.moderator).unwrap_or(false),
                        player_id: p.player_id,
                        team_id: p.team_id(),
                        team_captain: p
                            .team_id()
                            .and_then(|tid| teams.get(tid))
                            .map(|t| t.is_captain(p.player_id))
                            .unwrap_or(false),
                    })
                }
            })
            .collect();

        (player_dtos, real_players, real_players_live)
    }

    /// Gets initializer for new client.
    pub(crate) fn initializer(&self) -> PlayerUpdate {
        PlayerUpdate::Updated {
            added: Arc::clone(&self.previous),
            removed: Vec::new().into(),
            real_players: self.real_players_live as u32,
        }
    }

    /// Computes a diff, and updates cached dtos.
    pub(crate) fn delta(
        &mut self,
        teams: &TeamRepo<G>,
    ) -> Option<(Arc<[PlayerDto]>, Arc<[PlayerId]>, u32)> {
        let (current_players, real_players, real_players_live) = self.compute(teams);

        self.real_players = real_players;
        self.real_players_live = real_players_live;

        if let Some((added, removed)) =
            diff_large_n(&self.previous, &current_players, |dto| dto.player_id)
        {
            self.previous = current_players.into();
            Some((added.into(), removed.into(), real_players_live as u32))
        } else {
            None
        }
    }

    /// Cannot coincide with mutable references to players.
    pub fn borrow_player(&self, player_id: PlayerId) -> Option<AtomicRef<PlayerData<G>>> {
        self.get(player_id).map(|pt| pt.borrow_player())
    }

    /// Cannot coincide with other references to players.
    pub fn borrow_player_mut(&self, player_id: PlayerId) -> Option<AtomicRefMut<PlayerData<G>>> {
        self.get(player_id).map(|pt| pt.borrow_player_mut())
    }

    /// Iterates every player tuple (real and bot).
    pub fn iter(&self) -> impl Iterator<Item = &Arc<PlayerTuple<G>>> {
        self.players.values()
    }

    /// Iterates every player id (real and bot).
    pub fn iter_player_ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.players.keys().cloned()
    }

    /// Iterates every player tuple, immutably borrowing it automatically.
    /// Cannot coincide with mutable references to players.
    pub fn iter_borrow(&self) -> impl Iterator<Item = AtomicRef<PlayerData<G>>> {
        self.players.values().map(|pt| pt.borrow_player())
    }

    /// Iterates every player tuple, mutably borrowing it automatically.
    /// Cannot coincide with other references to players.
    pub fn iter_borrow_mut(&mut self) -> impl Iterator<Item = AtomicRefMut<PlayerData<G>>> {
        self.players.values().map(|pt| pt.borrow_player_mut())
    }
}

/// Player tuple contains the Player (real or bot) and the PlayerExtension.
///
/// The Player part is an [`AtomicRefCell`] because mutations are manually serialized.
///
/// The PlayerExtension part is an [`UnsafeCell`] because mutators are forced to hold a mutable reference
/// to a unique structure (such as the player's vehicle).
pub struct PlayerTuple<G: GameArenaService> {
    pub player: AtomicRefCell<PlayerData<G>>,
    pub extension: G::PlayerExtension,
}

impl<G: GameArenaService> PlayerTuple<G> {
    pub fn new(player: PlayerData<G>) -> Self {
        PlayerTuple {
            player: AtomicRefCell::new(player),
            extension: G::PlayerExtension::default(),
        }
    }
}

impl<G: GameArenaService> PlayerTuple<G> {
    /// Borrows the player.
    pub fn borrow_player(&self) -> AtomicRef<PlayerData<G>> {
        self.player.borrow()
    }

    /// Mutably borrows the player.
    pub fn borrow_player_mut(&self) -> AtomicRefMut<PlayerData<G>> {
        self.player.borrow_mut()
    }

    /// Borrows the player without checking for outstanding mutable borrows, the existence of which
    /// would cause undefined behavior.
    pub unsafe fn borrow_player_unchecked(&self) -> &PlayerData<G> {
        #[cfg(debug_assertions)]
        drop(self.borrow_player());
        &*self.player.as_ptr()
    }

    /// Mutably borrows the player without checking for outstanding borrows, the existence
    /// of which would cause undefined behavior.
    pub unsafe fn borrow_player_mut_unchecked(&self) -> &mut PlayerData<G> {
        #[cfg(debug_assertions)]
        drop(self.borrow_player_mut());
        &mut *self.player.as_ptr()
    }
}

impl<G: GameArenaService> PartialEq for PlayerTuple<G> {
    fn eq(&self, other: &Self) -> bool {
        self.player.as_ptr() == other.player.as_ptr()
    }
}

impl<G: GameArenaService> Debug for PlayerTuple<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.borrow_player().data)
    }
}

/// Data stored per real or bot player.
#[derive(Debug)]
pub struct PlayerData<G: GameArenaService> {
    pub player_id: PlayerId,
    pub score: u32,
    /// Whether the player was alive last time we checked.
    pub(crate) was_alive: bool,
    /// Whether the player was out of game last time we checked.
    pub(crate) was_out_of_game: bool,
    /// Whether the player was *ever* alive.
    was_ever_alive: bool,
    /// When was_alive was set to its current value.
    was_alive_timestamp: Instant,
    /// Present if the player has an active client a.k.a. session.
    pub client: Option<Box<PlayerClientData<G>>>,
    pub(crate) team: PlayerTeamData,
    pub data: G::PlayerData,
}

impl<G: GameArenaService> PlayerData<G> {
    pub fn new(player_id: PlayerId, client: Option<Box<PlayerClientData<G>>>) -> Self {
        Self {
            player_id,
            score: G::DEFAULT_SCORE,
            was_alive: false,
            was_out_of_game: false,
            was_ever_alive: false,
            was_alive_timestamp: Instant::now(),
            client,
            team: PlayerTeamData::default(),
            data: G::PlayerData::default(),
        }
    }

    /// Gets the player's current [`Alias`].
    pub fn alias(&self) -> PlayerAlias {
        if let Some(client_data) = self.client() {
            client_data.alias
        } else if self.is_bot() {
            PlayerAlias::from_bot_player_id(self.player_id)
        } else {
            debug_assert!(false, "impossible");
            G::default_alias()
        }
    }

    /// If player is a real player, returns their client data.
    pub fn client(&self) -> Option<&PlayerClientData<G>> {
        self.client.as_deref()
    }

    /// If player is a real player, returns a mutable reference to their client data.
    pub fn client_mut(&mut self) -> Option<&mut PlayerClientData<G>> {
        self.client.as_deref_mut()
    }

    /// Gets the player's current [`TeamId`].
    pub fn team_id(&self) -> Option<TeamId> {
        self.team.team_id()
    }

    /// Gets any invitation accepted by the player (always [`None`] for bots).
    pub fn invitation_accepted(&self) -> Option<&InvitationDto> {
        self.client()
            .and_then(|c| c.invitation.invitation_accepted.as_ref())
    }

    /// A lagging indicator of [`GameArenaService::is_alive`], updated after the game runs.
    pub(crate) fn is_alive(&self) -> bool {
        self.was_alive
    }

    /// If the player was recently alive, this returns how long they were alive for.
    pub(crate) fn alive_duration(&self) -> Option<Duration> {
        self.was_alive.then(|| self.was_alive_timestamp.elapsed())
    }

    /// If the player was recently not alive, this returns how long they were not alive for.
    ///
    pub(crate) fn not_alive_duration(&self) -> Option<Duration> {
        (!self.was_alive).then(|| self.was_alive_timestamp.elapsed())
    }

    /// Returns true iff player is a bot (their id is a bot id).
    pub fn is_bot(&self) -> bool {
        self.player_id.is_bot()
    }

    /// Returns true iff the player 1) never played yet 2) stopped playing over half a minute ago.
    pub fn is_out_of_game(&self) -> bool {
        !self.was_ever_alive
            || self.not_alive_duration().unwrap_or(Duration::ZERO) > Duration::from_secs(30)
    }
}

impl<G: GameArenaService> Deref for PlayerData<G> {
    type Target = G::PlayerData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<G: GameArenaService> DerefMut for PlayerData<G> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
