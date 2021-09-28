// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::session::Session;
use crate::team::Team;
use core_protocol::dto::{
    LeaderboardDto, LiveboardDto, MemberDto, MessageDto, PlayerDto, RegionDto, TeamDto,
};
use core_protocol::id::PeriodId;
use core_protocol::id::*;
use enum_iterator::IntoEnumIterator;
use log::{debug, trace};
use ringbuffer::RingBufferExt;
use std::collections::hash_map::HashMap;
use std::mem;
use std::sync::Arc;

pub struct Repo {
    // Assume these fields are synchronized via Actor so Mutex is not required.
    pub arenas: HashMap<ArenaId, Arena>,
    pub players: HashMap<PlayerId, SessionId>,
}

impl Repo {
    /// How long after play stops before session is no longer live, e.g. promote a different captain.
    pub const DYING_DURATION_MILLIS: u64 = 30000; // half minute.

    pub fn new() -> Self {
        Repo {
            arenas: HashMap::new(),
            players: HashMap::new(),
        }
    }

    // Newbie client needs initializers.
    pub fn get_initializers(
        &mut self,
        arena_id: ArenaId,
    ) -> Option<(
        Vec<(Arc<[LeaderboardDto]>, PeriodId)>,
        Arc<[LiveboardDto]>,
        Arc<[MessageDto]>,
        Arc<[PlayerDto]>,
        Arc<[TeamDto]>,
    )> {
        debug!("get_initializers()");

        if let Some(arena) = self.arenas.get(&arena_id) {
            let mut leaderboards: Vec<(Arc<[LeaderboardDto]>, PeriodId)> = vec![];
            for period in PeriodId::into_enum_iter() {
                leaderboards.push((arena.leaderboards[period as usize].clone().into(), period));
            } // for leaderboard_period
            let leaderboard_initializer = leaderboards.into();

            let (liveboard, _) = arena.get_liveboard(true);
            let liveboard_initializer = liveboard.into();

            let message_initializer = arena
                .newbie_messages
                .iter()
                .map(|rc| MessageDto::clone(&rc))
                .collect();

            let mut players = vec![];
            for session in arena.sessions.values() {
                if !session.live {
                    continue;
                }
                if let Some(play) = session.plays.last() {
                    players.push(PlayerDto {
                        alias: session.alias.clone(),
                        player_id: session.player_id,
                        team_captain: play.team_captain,
                        team_id: play.team_id,
                    });
                }
            }
            let players_initializer = players.into();

            let mut teams = vec![];
            for (team_id, Team { team_name, .. }) in arena.teams.iter() {
                teams.push(TeamDto {
                    team_id: *team_id,
                    team_name: team_name.clone(),
                });
            }
            let teams_initializer = teams.into();

            Some((
                leaderboard_initializer,
                liveboard_initializer,
                message_initializer,
                players_initializer,
                teams_initializer,
            ))
        } else {
            None
        }
    }

    // Returns the liveboards for all arenas.
    pub fn get_liveboards(&self, include_bots: bool) -> Vec<(ArenaId, GameId, Vec<LiveboardDto>)> {
        self.arenas
            .iter()
            .map(|(arena_id, arena)| {
                (
                    *arena_id,
                    arena.game_id,
                    arena.get_liveboard(include_bots).0,
                )
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
            let server_addr = arena.server_addr.clone();

            regions.push(RegionDto {
                player_count,
                region_id,
                server_addr,
            });
        }

        regions.into()
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

    // Assume caller reads public updates and broadcasts to all clients.
    pub fn read_broadcasts(
        &mut self,
    ) -> Option<(
        Vec<(ArenaId, (Arc<[PlayerDto]>, Arc<[PlayerId]>))>,
        Vec<(ArenaId, (Arc<[TeamDto]>, Arc<[TeamId]>))>,
    )> {
        trace!("read_broadcasts()");

        // ARC is used because the same message is sent to multiple observers.
        let mut players_added_or_removed: Vec<(ArenaId, (Arc<[PlayerDto]>, Arc<[PlayerId]>))> =
            Vec::new();
        let mut teams_added_or_removed: Vec<(ArenaId, (Arc<[TeamDto]>, Arc<[TeamId]>))> =
            Vec::new();

        for (arena_id, arena) in Arena::iter_mut(&mut self.arenas) {
            if !(arena.broadcast_players.add.is_empty()
                && arena.broadcast_players.remove.is_empty())
            {
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
                                    alias: session.alias.clone(),
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

                players_added_or_removed.push((*arena_id, (added.into(), removed.into())));
            }

            if !(arena.broadcast_teams.add.is_empty() && arena.broadcast_teams.remove.is_empty()) {
                let mut added = vec![];
                if !arena.broadcast_teams.add.is_empty() {
                    debug!("teams_added");
                    for team_id in arena.broadcast_teams.add.iter() {
                        if let Some(Team { team_name, .. }) = arena.teams.get(&team_id) {
                            added.push(TeamDto {
                                team_id: *team_id,
                                team_name: team_name.clone(),
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

        if players_added_or_removed.is_empty() && teams_added_or_removed.is_empty() {
            None
        } else {
            Some((players_added_or_removed, teams_added_or_removed))
        }
    }

    // Assume this is called periodically to read changes in the leaderboards.
    pub fn read_leaderboards(&mut self) -> Option<Vec<(ArenaId, Arc<[LeaderboardDto]>, PeriodId)>> {
        // ARC is used because the same message is sent to multiple observers.
        let mut changed_leaderboards: Vec<(ArenaId, Arc<[LeaderboardDto]>, PeriodId)> = vec![];
        for (arena_id, arena) in Arena::iter_mut(&mut self.arenas) {
            for period in PeriodId::into_enum_iter() {
                if !arena.leaderboard_changed[period as usize] {
                    continue;
                }
                trace!(
                    "leaderboard_changed for arena {:?} period {:?}",
                    arena_id,
                    period
                );
                changed_leaderboards.push((
                    *arena_id,
                    arena.leaderboards[period as usize].clone().into(),
                    period,
                ));
                arena.leaderboard_changed[period as usize] = false;
            } // for leaderboard_period
        }

        if changed_leaderboards.is_empty() {
            None
        } else {
            Some(changed_leaderboards)
        }
    }

    // Assume this is called periodically to read changes in the liveboards.
    pub fn read_liveboards(&mut self) -> Option<Vec<(ArenaId, Arc<[LiveboardDto]>)>> {
        // ARC is used because the same message is sent to multiple observers.
        let mut changed_liveboards: Vec<(ArenaId, Arc<[LiveboardDto]>)> = vec![];
        for (arena_id, arena) in Arena::iter_mut(&mut self.arenas) {
            if !arena.liveboard_changed {
                continue;
            }
            trace!("liveboard_changed for arena {:?}", arena_id);
            arena.liveboard_changed = false;
            let (leaderboard, min_score) = arena.get_liveboard(true);
            arena.liveboard_min_score = min_score;
            changed_liveboards.push((*arena_id, leaderboard.into()));
        }

        if changed_liveboards.is_empty() {
            None
        } else {
            Some(changed_liveboards)
        }
    }

    // Assume caller reads changes to notify servers.
    pub fn read_server_updates(&mut self) -> Option<Vec<(ArenaId, Arc<[MemberDto]>)>> {
        trace!("read_server_updates()");

        let mut result: Vec<(ArenaId, Arc<[MemberDto]>)> = vec![];
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
        #[allow(unused_mut)]
        let mut joiners_added_or_removed: (Arc<[PlayerId]>, Arc<[PlayerId]>) =
            (Vec::new().into(), Vec::new().into());
        #[allow(unused_mut)]
        let mut joins_added_or_removed: (Arc<[TeamId]>, Arc<[TeamId]>) =
            (Vec::new().into(), Vec::new().into());
        let mut messages_added: Arc<[MessageDto]> = Vec::new().into();

        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, &session_id) {
                messages_added = mem::take(&mut session.inbox)
                    .iter()
                    .map(|rc| MessageDto::clone(&rc))
                    .collect();
                if let Some(play) = session.plays.last_mut() {
                    joiners_added_or_removed = (
                        mem::take(&mut play.whisper_joiners.add)
                            .into_iter()
                            .collect(),
                        mem::take(&mut play.whisper_joiners.remove)
                            .into_iter()
                            .collect(),
                    );
                    joins_added_or_removed = (
                        mem::take(&mut play.whisper_joins.add).into_iter().collect(),
                        mem::take(&mut play.whisper_joins.remove)
                            .into_iter()
                            .collect(),
                    );
                }
            }
        }

        (
            joiners_added_or_removed,
            joins_added_or_removed,
            messages_added,
        )
    }
}
