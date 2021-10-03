// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::generate_id::generate_id;
use crate::notify_set::NotifySet;
use crate::repo::Repo;
use crate::session::{Play, Session};
use crate::team::Team;
use core_protocol::dto::{LeaderboardDto, LiveboardDto, MessageDto, RulesDto};
use core_protocol::id::*;
use core_protocol::UnixTime;
use core_protocol::*;
use log::debug;
use ringbuffer::ConstGenericRingBuffer;
use std::collections::hash_map::{Entry, HashMap};
use std::rc::Rc;
use std::sync::Arc;

// Eventually, each arena will be able to contain one or more scenes.
#[allow(dead_code)]
pub struct Arena {
    pub broadcast_players: NotifySet<SessionId>,
    pub broadcast_teams: NotifySet<TeamId>,
    pub confide_membership: HashMap<PlayerId, Option<TeamId>>, // For game server.
    pub date_created: UnixTime,
    pub date_start: UnixTime,
    pub date_stop: Option<UnixTime>,
    pub game_id: GameId,
    pub leaderboards: [Arc<[LeaderboardDto]>; PeriodId::VARIANT_COUNT],
    pub leaderboard_changed: [bool; PeriodId::VARIANT_COUNT],
    pub liveboard_changed: bool,
    // Minimum score to appear on liveboard.
    pub liveboard_min_score: u32,
    pub newbie_messages: ConstGenericRingBuffer<Rc<MessageDto>, 16>,
    pub region_id: RegionId,
    pub rules: RulesDto,
    pub sessions: HashMap<SessionId, Session>,
    pub server_id: ServerId, // e.g. "s1.usa.softbear.com"
    pub teams: HashMap<TeamId, Team>,
}

impl Arena {
    pub fn new(game_id: GameId, region_id: RegionId, rules: RulesDto, server_id: ServerId) -> Self {
        let date_created = get_unix_time_now();
        Self {
            broadcast_players: NotifySet::new(),
            broadcast_teams: NotifySet::new(),
            confide_membership: HashMap::new(),
            date_created,
            date_start: date_created,
            date_stop: None,
            game_id,
            leaderboards: [Vec::new().into(), Vec::new().into(), Vec::new().into()],
            leaderboard_changed: [false; PeriodId::VARIANT_COUNT],
            liveboard_changed: false,
            liveboard_min_score: 0,
            newbie_messages: ConstGenericRingBuffer::new(),
            region_id,
            rules,
            sessions: HashMap::new(),
            server_id,
            teams: HashMap::new(),
        }
    }

    /// If there exists a session in `sessions` that is captain of the specified
    /// `team_id` then return its `session_id`.  Otherwise, return `None`.
    pub fn static_captain_of_team(
        sessions: &HashMap<SessionId, Session>,
        team_id: &TeamId,
    ) -> Option<SessionId> {
        let mut captain_session_id: Option<SessionId> = None;
        for (session_id, session) in sessions.iter() {
            if session.date_terminated.is_some() || !session.live {
                continue;
            }
            if let Some(play) = session.plays.last() {
                if !play.team_captain {
                    continue;
                }
                if let Some(captain_team_id) = play.team_id {
                    if captain_team_id == *team_id {
                        captain_session_id = Some(*session_id);
                        break;
                    }
                }
            }
        }

        captain_session_id
    }

    /// If there exists a session in the arena that is captain of the specified
    /// `team_id` then return its `session_id`.  Otherwise, return `None`.
    pub fn captain_of_team(&self, team_id: &TeamId) -> Option<SessionId> {
        Self::static_captain_of_team(&self.sessions, team_id)
    }

    pub fn get_liveboard(&self, include_bots: bool) -> (Vec<LiveboardDto>, u32) {
        // TODO: Collect into heap for better performance.
        let mut liveboard = Vec::new();
        for session in self.sessions.values() {
            if session.bot && !include_bots {
                continue;
            }
            if !session.live {
                continue;
            }
            if let Some(play) = session.plays.last() {
                if play.date_stop.is_some() {
                    // Even if session remains live, remove from liveboard when play stops.
                    continue;
                }
                if let Some(score) = play.score {
                    liveboard.push(LiveboardDto {
                        team_captain: play.team_captain,
                        team_id: play.team_id,
                        player_id: session.player_id,
                        score,
                    });
                }
            }
        }
        liveboard.sort_by(|a, b| b.score.cmp(&a.score));
        liveboard.truncate(10);
        let min_score = if liveboard.len() == 10 {
            liveboard.last().unwrap().score
        } else {
            0
        };
        (liveboard, min_score)
    }

    pub fn get_mut<'a>(
        arenas: &'a mut HashMap<ArenaId, Arena>,
        arena_id: &'a ArenaId,
    ) -> Option<&'a mut Arena> {
        let mut result = None;
        if let Some(arena) = arenas.get_mut(arena_id) {
            if arena.date_stop.is_none() {
                result = Some(arena);
            }
        }
        result
    }

    pub fn iter_mut(
        arenas: &mut HashMap<ArenaId, Arena>,
    ) -> impl Iterator<Item = (&ArenaId, &mut Arena)> {
        arenas.iter_mut().filter_map(move |(arena_id, arena)| {
            if arena.date_stop.is_none() {
                Some((arena_id, arena))
            } else {
                None
            }
        })
    }

    /// If the specified `session_id` is a captain then return its `team_id`, otherwise return `None`.
    pub fn team_of_captain(&mut self, session_id: SessionId) -> Option<(TeamId, &mut Play)> {
        let mut result = None;
        if let Some(session) = Session::get_mut(&mut self.sessions, session_id) {
            if session.live {
                if let Some(play) = session.plays.last_mut() {
                    if play.team_captain {
                        if let Some(team_id) = play.team_id {
                            result = Some((team_id, play));
                        }
                    }
                }
            }
        }

        result
    }
}

impl Repo {
    // Server (re)starts arena when the executable is run.
    pub fn start_arena(
        &mut self,
        game_id: GameId,
        region_id: RegionId,
        rules: Option<RulesDto>,
        saved_arena_id: Option<ArenaId>,
        server_id: ServerId,
    ) -> ArenaId {
        let mut result: Option<ArenaId> = None;
        if let Some(arena_id) = saved_arena_id {
            if let Some(arena) = self.arenas.get_mut(&arena_id) {
                arena.date_start = get_unix_time_now();
                arena.date_stop = None;
                result = Some(arena_id);
            }
        }

        if result.is_none() {
            result = Some(loop {
                let arena_id = ArenaId(generate_id());
                if let Entry::Vacant(e) = self.arenas.entry(arena_id) {
                    e.insert(Arena::new(
                        game_id,
                        region_id,
                        rules.unwrap_or_default(),
                        server_id,
                    ));
                    break arena_id;
                }
            });
        }

        result.unwrap()
    }

    // Server reports it has stopped arena.
    pub fn stop_arena(&mut self, arena_id: ArenaId) {
        debug!("stop_arena(arena={:?})", arena_id);
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            arena.date_stop = Some(get_unix_time_now());

            // Clear flags.
            arena.liveboard_changed = false;
            arena.broadcast_players.add.clear();
            arena.broadcast_players.remove.clear();
            arena.broadcast_teams.add.clear();
            arena.broadcast_teams.remove.clear();

            // Terminate all sessions (which also stops all plays).
            for (_, session) in Session::iter_mut(&mut arena.sessions) {
                session.terminate_session();
            }
        }
    }
}
