// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::invitation::Invitation;
use crate::session::Session;
use crate::team::Team;
// TODO: use chrono::{DateTime, Utc};
use chrono::Timelike;
use chrono::Utc;
use core_protocol::dto::{
    LeaderboardDto, LiveboardDto, MemberDto, MessageDto, PlayerDto, RegionDto, RestartDto, TeamDto,
};
use core_protocol::id::PeriodId;
use core_protocol::id::*;
use enum_iterator::IntoEnumIterator;
use log::{debug, trace};
use ringbuffer::RingBufferExt;
use std::collections::hash_map::HashMap;
use std::mem;
use std::sync::Arc;
use sysinfo::{RefreshKind, System, SystemExt};

pub struct Repo {
    // Assume these fields are synchronized via Actor so Mutex is not required.
    pub arenas: HashMap<ArenaId, Arena>,
    pub armageddon: bool,
    pub invitations: HashMap<InvitationId, Invitation>,
    pub players: HashMap<PlayerId, SessionId>,
    pub prior_liveboard: Vec<LiveboardDto>,
    pub stopping: Option<RestartDto>,
    pub system_status: System,
}

impl Default for Repo {
    fn default() -> Self {
        Self::new()
    }
}

impl Repo {
    /// How long after play stops before session is no longer live, e.g. promote a different captain.
    pub const DYING_DURATION_MILLIS: u64 = 30000; // half minute.

    pub fn new() -> Self {
        Repo {
            arenas: HashMap::new(),
            armageddon: false,
            invitations: HashMap::new(),
            players: HashMap::new(),
            prior_liveboard: vec![].into(),
            stopping: None,
            system_status: System::new_with_specifics(RefreshKind::new().with_cpu().with_memory()),
        }
    }

    // Newbie client needs initializers.
    #[allow(clippy::type_complexity)]
    pub fn get_initializers(
        &mut self,
        arena_id: ArenaId,
    ) -> Option<(
        [Arc<[LeaderboardDto]>; PeriodId::VARIANT_COUNT],
        Arc<[LiveboardDto]>,
        Arc<[MessageDto]>,
        (u32, Arc<[PlayerDto]>),
        Arc<[TeamDto]>,
    )> {
        debug!("get_initializers()");

        if let Some(arena) = self.arenas.get(&arena_id) {
            let leaderboard_initializer = arena.leaderboards.clone();

            let (liveboard, _, _) = arena.get_liveboard(arena.rules.show_bots_on_liveboard);
            let liveboard_initializer = liveboard.into();

            let message_initializer = arena
                .newbie_messages
                .iter()
                .map(|rc| MessageDto::clone(rc))
                .collect();

            let mut player_count = 0;
            let mut players = vec![];
            for session in arena.sessions.values() {
                if !session.live {
                    continue;
                }
                if let Some(play) = session.plays.last() {
                    if !session.bot {
                        player_count += 1;
                    }

                    players.push(PlayerDto {
                        alias: session.alias,
                        player_id: session.player_id,
                        team_captain: play.team_captain,
                        team_id: play.team_id,
                    });
                }
            }
            let players_initializer = players.into();

            let mut teams = vec![];
            for (&team_id, Team { team_name, .. }) in arena.teams.iter() {
                teams.push(TeamDto {
                    team_id,
                    team_name: *team_name,
                });
            }
            let teams_initializer = teams.into();

            Some((
                leaderboard_initializer,
                liveboard_initializer,
                message_initializer,
                (player_count, players_initializer),
                teams_initializer,
            ))
        } else {
            None
        }
    }

    // Returns the liveboards for all arenas.
    pub fn get_liveboards(
        &self,
        include_bots: bool,
    ) -> Vec<(ArenaId, GameId, Vec<LiveboardDto>, bool)> {
        self.arenas
            .iter()
            .map(|(arena_id, arena)| {
                let (liveboard, _, leaderboard_worthy) = arena.get_liveboard(include_bots);
                (*arena_id, arena.game_id, liveboard, leaderboard_worthy)
            })
            .collect()
    }

    // Returns the list of regions and the number of players in each.
    pub fn get_regions(&mut self) -> Arc<[RegionDto]> {
        let mut regions = Vec::new();
        for (_, arena) in self.arenas.iter() {
            if arena.date_stop.is_some() {
                continue;
            }
            let mut player_count = 0;
            for (_, session) in arena.sessions.iter() {
                if session.bot || session.date_terminated.is_some() || !session.live {
                    continue;
                }
                player_count += 1;
            }

            let region_id = arena.region_id;
            let server_id = arena.server_id;

            regions.push(RegionDto {
                player_count,
                region_id,
                server_id,
            });
        }

        regions.into()
    }

    fn get_usage(&self) -> (u32, u32) {
        let mut player_count = 0;
        let mut max_score = 0;
        for (_, arena) in self.arenas.iter() {
            if arena.date_stop.is_some() {
                continue;
            }
            for (_, session) in arena.sessions.iter() {
                if !session.live || session.bot {
                    continue;
                }
                player_count += 1;
                if let Some(play) = session.plays.last() {
                    if let Some(score) = play.score {
                        if score > max_score {
                            max_score = score;
                        }
                    }
                }
            }
        }
        (player_count, max_score)
    }

    /// Assume caller uses this method to check if repo can be stopped.
    pub fn is_stoppable(&self) -> bool {
        let mut stoppable = false;
        if let Some(conditions) = self.stopping {
            stoppable = true;

            let now = Utc::now();
            let hour = now.hour();

            if conditions.min_hour > hour || conditions.max_hour < hour {
                stoppable = false;
            }

            let (player_count, max_score) = self.get_usage();
            if let Some(max_players_allowed) = conditions.max_players {
                if player_count > max_players_allowed {
                    stoppable = false;
                }
            }
            if let Some(max_score_allowed) = conditions.max_score {
                if max_score > max_score_allowed {
                    stoppable = false;
                }
            }
        }

        stoppable
    }

    /// Assume caller uses this method to populate cache with leaderboards.
    pub fn put_leaderboard(
        &mut self,
        arena_id: ArenaId,
        leaderboard: Arc<[LeaderboardDto]>,
        period: PeriodId,
    ) {
        if let Some(arena) = self.arenas.get_mut(&arena_id) {
            if arena.leaderboards[period as usize] != leaderboard {
                arena.leaderboards[period as usize] = leaderboard;
                arena.leaderboard_changed[period as usize] = true;
            }
        }
    }

    /// Assume caller polls this method to check when to start "armageddon"
    /// (bots outnumber and eliminate players, as a prelude to server shutdown).
    pub fn read_armageddon(&mut self) -> bool {
        let armageddon = self.armageddon;
        if armageddon {
            self.armageddon = false;
        }
        armageddon
    }

    // Assume caller reads public updates and broadcasts to all clients.
    #[allow(clippy::type_complexity)]
    pub fn read_broadcasts(
        &mut self,
    ) -> Option<(
        Vec<(ArenaId, (u32, Arc<[PlayerDto]>, Arc<[PlayerId]>))>,
        Vec<(ArenaId, (Arc<[TeamDto]>, Arc<[TeamId]>))>,
    )> {
        trace!("read_broadcasts()");

        // ARC is used because the same message is sent to multiple observers.
        let mut players_counted_added_or_removed = vec![];
        let mut teams_added_or_removed = vec![];

        for (arena_id, arena) in Arena::iter_mut(&mut self.arenas) {
            if !(arena.broadcast_players.add.is_empty()
                && arena.broadcast_players.remove.is_empty())
            {
                let mut player_count = 0;

                for (_, session) in arena.sessions.iter() {
                    if session.live && !session.bot {
                        player_count += 1;
                    }
                }

                let mut added = vec![];
                let mut count = 0;

                if !arena.broadcast_players.add.is_empty() {
                    for session_id in arena.broadcast_players.add.iter() {
                        if let Some(session) = arena.sessions.get(session_id) {
                            if !session.bot {
                                count += 1;
                            }
                            if let Some(play) = session.plays.last() {
                                added.push(PlayerDto {
                                    alias: session.alias,
                                    player_id: session.player_id,
                                    team_captain: play.team_captain,
                                    team_id: play.team_id,
                                });
                            }
                        }
                    }
                    arena.broadcast_players.add.clear();
                    if count != 0 {
                        debug!("{} players_added", count);
                    }
                }

                let mut removed = vec![];
                if !arena.broadcast_players.remove.is_empty() {
                    let mut count = 0;
                    for session_id in arena.broadcast_players.remove.iter() {
                        if let Some(session) = arena.sessions.get(session_id) {
                            if !session.bot {
                                count += 1;
                            }
                            removed.push(session.player_id);
                        }
                    }
                    arena.broadcast_players.remove.clear();
                    if count != 0 {
                        debug!("{} players_removed", count);
                    }
                }

                players_counted_added_or_removed
                    .push((*arena_id, (player_count, added.into(), removed.into())));
            }

            if !(arena.broadcast_teams.add.is_empty() && arena.broadcast_teams.remove.is_empty()) {
                let mut added = vec![];
                if !arena.broadcast_teams.add.is_empty() {
                    debug!("teams_added");
                    for team_id in arena.broadcast_teams.add.iter() {
                        if let Some(Team { team_name, .. }) = arena.teams.get(team_id) {
                            added.push(TeamDto {
                                team_id: *team_id,
                                team_name: *team_name,
                            });
                        }
                    }
                    arena.broadcast_teams.add.clear();
                }

                let mut removed = vec![];
                if !arena.broadcast_teams.remove.is_empty() {
                    debug!("teams_removed");
                    for team_id in arena.broadcast_teams.remove.iter() {
                        removed.push(*team_id);
                    }
                    arena.broadcast_teams.remove.clear();
                }
                teams_added_or_removed.push((*arena_id, (added.into(), removed.into())));
            }
        }

        if players_counted_added_or_removed.is_empty() && teams_added_or_removed.is_empty() {
            None
        } else {
            Some((players_counted_added_or_removed, teams_added_or_removed))
        }
    }

    // Assume this is called periodically to read changes in the leaderboards.
    // ARC is used because the same message is sent to multiple observers.
    #[allow(clippy::type_complexity)]
    pub fn read_leaderboards(
        &mut self,
    ) -> impl Iterator<Item = (ArenaId, Arc<[LeaderboardDto]>, PeriodId)> + '_ {
        Arena::iter_mut(&mut self.arenas).flat_map(|(&arena_id, arena)| {
            PeriodId::into_enum_iter().filter_map(move |period| {
                if !arena.leaderboard_changed[period as usize] {
                    return None;
                }
                arena.leaderboard_changed[period as usize] = false;

                trace!(
                    "leaderboard_changed for arena {:?} period {:?}",
                    arena_id,
                    period
                );

                Some((
                    arena_id,
                    arena.leaderboards[period as usize].clone(),
                    period,
                ))
            })
        })
    }

    // Assume this is called periodically to read changes in the liveboards.
    // ARC is used because the same message is sent to multiple observers.
    pub fn read_liveboards(
        &mut self,
    ) -> impl Iterator<Item = (ArenaId, Arc<[LiveboardDto]>, Arc<[PlayerId]>)> + '_ {
        let prior_liveboard = &mut self.prior_liveboard;

        Arena::iter_mut(&mut self.arenas).filter_map(move |(arena_id, arena)| {
            if !arena.liveboard_changed {
                return None;
            }
            arena.liveboard_changed = false;

            trace!("liveboard_changed for arena {:?}", arena_id);

            let (current_liveboard, min_score, _) =
                arena.get_liveboard(arena.rules.show_bots_on_liveboard);
            arena.liveboard_min_score = min_score;

            let mut added = vec![];
            let mut removed = vec![];

            for old_item in prior_liveboard.iter() {
                if let Some(new_item) = current_liveboard
                    .iter()
                    .find(|i| i.player_id == old_item.player_id)
                {
                    // Add changed items.
                    if new_item != old_item {
                        added.push(new_item.clone());
                    }
                } else {
                    // Remove missing items.
                    removed.push(old_item.player_id);
                }
            }

            // Add new items.
            for new_item in current_liveboard.iter() {
                if prior_liveboard
                    .iter()
                    .find(|i| i.player_id == new_item.player_id)
                    .is_none()
                {
                    added.push(new_item.clone());
                }
            }

            *prior_liveboard = current_liveboard;

            // No changes.
            if added.is_empty() && removed.is_empty() {
                return None;
            }

            Some((*arena_id, added.into(), removed.into()))
        })
    }

    // Assume caller reads changes to notify servers.
    pub fn read_server_updates(&mut self) -> Option<Vec<(ArenaId, Arc<[MemberDto]>)>> {
        trace!("read_server_updates()");

        let mut result = vec![];
        for (arena_id, arena) in Arena::iter_mut(&mut self.arenas) {
            let mut team_assignments = vec![];
            if !arena.confide_membership.is_empty() {
                for (player_id, team_id) in arena.confide_membership.drain() {
                    team_assignments.push(MemberDto { player_id, team_id });
                }
            }
            if !team_assignments.is_empty() {
                result.push((*arena_id, team_assignments.into()));
            }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    // Assume caller reads private updates and whispers to the appropriate client.
    #[allow(clippy::type_complexity)]
    pub fn read_whispers(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
    ) -> (
        (Arc<[PlayerId]>, Arc<[PlayerId]>),
        (Arc<[TeamId]>, Arc<[TeamId]>),
        Arc<[MessageDto]>,
    ) {
        trace!(
            "read_whispers(arena={:?}, session={:?})",
            arena_id,
            session_id
        );

        // TODO: the return values are used only once; consider using Box<> instead of Arc<>.
        let mut joiners_added_or_removed = (vec![].into(), vec![].into());
        let mut joins_added_or_removed = (vec![].into(), vec![].into());
        let mut messages_added = vec![].into();

        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                messages_added = mem::take(&mut session.inbox)
                    .iter()
                    .map(|rc| MessageDto::clone(rc))
                    .collect();
                joiners_added_or_removed = (
                    mem::take(&mut session.whisper_joiners.add)
                        .into_iter()
                        .collect(),
                    mem::take(&mut session.whisper_joiners.remove)
                        .into_iter()
                        .collect(),
                );
                joins_added_or_removed = (
                    mem::take(&mut session.whisper_joins.add)
                        .into_iter()
                        .collect(),
                    mem::take(&mut session.whisper_joins.remove)
                        .into_iter()
                        .collect(),
                );
            }
        }

        (
            joiners_added_or_removed,
            joins_added_or_removed,
            messages_added,
        )
    }

    /// Set the stop conditions that, when met, will cause the service to exit.
    pub fn set_stop_conditions(&mut self, condition: RestartDto) {
        if self.stopping.is_none() {
            self.armageddon = true;
            self.stopping = Some(condition);
        }
    }
}
