// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::generate_id::generate_id;
use crate::notify_set::NotifySet;
use crate::repo::Repo;
use crate::session::Session;
use crate::team::Team;
use core_protocol::dto::{LeaderboardDto, LiveboardDto, MessageDto, RulesDto};
use core_protocol::id::*;
use core_protocol::UnixTime;
use core_protocol::*;
use log::debug;
use ringbuffer::ConstGenericRingBuffer;
use std::collections::hash_map::{Entry, HashMap};
use std::collections::BinaryHeap;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;

// Eventually, each arena will be able to contain one or more scenes.
#[allow(dead_code)]
pub struct Arena {
    pub armageddon: bool,
    pub broadcast_players: NotifySet<SessionId>,
    pub broadcast_teams: NotifySet<TeamId>,
    pub confide_membership: HashMap<PlayerId, Option<TeamId>>, // For game server.
    pub date_created: UnixTime,
    pub date_put: UnixTime,
    pub date_start: UnixTime,
    pub date_stop: Option<UnixTime>,
    pub game_id: GameId,
    pub leaderboards: [Arc<[LeaderboardDto]>; PeriodId::VARIANT_COUNT],
    pub leaderboard_changed: [bool; PeriodId::VARIANT_COUNT],
    pub liveboard_changed: bool,
    // Minimum score to appear on liveboard.
    pub liveboard_min_score: u32,
    pub newbie_messages: ConstGenericRingBuffer<Rc<MessageDto>, 16>,
    pub other_server: bool,
    pub region_id: RegionId,
    pub rules: RulesDto,
    pub sessions: HashMap<SessionId, Session>,
    pub server_id: Option<ServerId>,
    pub teams: HashMap<TeamId, Team>,
    /// Updates per second, as reported by the game server.
    pub ups: Option<f32>,
}

impl Arena {
    pub fn new(
        game_id: GameId,
        region_id: RegionId,
        rules: RulesDto,
        server_id: Option<ServerId>,
    ) -> Self {
        let date_created = get_unix_time_now();
        Self {
            armageddon: false,
            broadcast_players: NotifySet::new(),
            broadcast_teams: NotifySet::new(),
            confide_membership: HashMap::new(),
            date_created,
            date_put: date_created,
            date_start: date_created,
            date_stop: None,
            game_id,
            leaderboards: [Vec::new().into(), Vec::new().into(), Vec::new().into()],
            leaderboard_changed: [false; PeriodId::VARIANT_COUNT],
            liveboard_changed: false,
            liveboard_min_score: 0,
            newbie_messages: ConstGenericRingBuffer::new(),
            other_server: false,
            region_id,
            rules,
            sessions: HashMap::new(),
            server_id,
            teams: HashMap::new(),
            ups: None,
        }
    }

    /// Tracks sessions that are on another server.
    pub fn create_other_server(game_id: GameId) -> Self {
        // The region, rules and most other fields are ignored.
        let mut arena = Self::new(game_id, RegionId::default(), RulesDto::default(), None);
        arena.other_server = true;
        arena
    }

    /// If there exists a session in `sessions` that is captain of the specified
    /// `team_id` then return its `session_id`.  Otherwise, return `None`.
    pub fn static_captain_of_team(
        sessions: &HashMap<SessionId, Session>,
        team_id: TeamId,
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
                    if captain_team_id == team_id {
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
    pub fn captain_of_team(&self, team_id: TeamId) -> Option<SessionId> {
        Self::static_captain_of_team(&self.sessions, team_id)
    }

    /// Returns a tuple consisting of the liveboard, the minimum score on the liveboard, and
    /// a boolean representing if the liveboard scores are worthy of leaderboard placement.
    pub fn get_liveboard(&self, include_bots: bool) -> (Vec<LiveboardDto>, u32, bool) {
        let mut liveboard = BinaryHeap::new();
        let mut real_players = 0;

        liveboard.extend(self.sessions.values().filter_map(|session| {
            if !session.live {
                return None;
            }
            if let Some(play) = session.plays.last() {
                // Pay already over.
                if play.date_stop.is_some() {
                    // Even if session remains live, remove from liveboard when play stops.
                    return None;
                }
                if !session.bot {
                    real_players += 1;
                } else if !include_bots {
                    return None;
                }
                play.score.map(|score| LiveboardDto {
                    team_captain: play.team_captain,
                    team_id: play.team_id,
                    player_id: session.player_id,
                    score,
                })
            } else {
                None
            }
        }));

        let liveboard: Vec<_> = liveboard.into_iter_sorted().take(10).collect();

        // If there isn't a 10th item anyone is qualified no matter their score.
        let min_score = liveboard.get(10).map(|i| i.score).unwrap_or(0);

        let leaderboard_worthy = real_players >= self.rules.leaderboard_min_players;
        (liveboard, min_score, leaderboard_worthy)
    }

    /// Checks if arena is valid in trivial cases (where stopped/other_server arenas don't matter).
    fn valid(&self) -> bool {
        self.date_stop.is_none() && !self.other_server
    }

    pub fn get(arenas: &HashMap<ArenaId, Arena>, arena_id: ArenaId) -> Option<&Arena> {
        arenas.get(&arena_id).filter(|arena| arena.valid())
    }

    pub fn get_mut(arenas: &mut HashMap<ArenaId, Arena>, arena_id: ArenaId) -> Option<&mut Arena> {
        arenas.get_mut(&arena_id).filter(|arena| arena.valid())
    }

    pub fn iter(arenas: &HashMap<ArenaId, Arena>) -> impl Iterator<Item = (ArenaId, &Arena)> {
        arenas
            .iter()
            .filter_map(move |(&arena_id, arena)| arena.valid().then_some((arena_id, arena)))
    }

    pub fn iter_mut(
        arenas: &mut HashMap<ArenaId, Arena>,
    ) -> impl Iterator<Item = (ArenaId, &mut Arena)> {
        arenas
            .iter_mut()
            .filter_map(move |(&arena_id, arena)| arena.valid().then_some((arena_id, arena)))
    }

    /// If the specified `session_id` is a captain then return its `team_id`, otherwise return `None`.
    pub fn team_of_captain(&mut self, session_id: SessionId) -> Option<(TeamId, &mut Session)> {
        let mut result = None;
        if let Some(session) = Session::get_mut(&mut self.sessions, session_id) {
            if session.live {
                if let Some(play) = session.plays.last() {
                    if play.team_captain {
                        if let Some(team_id) = play.team_id {
                            result = Some((team_id, session));
                        }
                    }
                }
            }
        }

        result
    }
}

impl Repo {
    /// Assume this is called frequently to prune other_server arenas that are unused for 5 secs.
    pub fn prune_arenas(&mut self) {
        let now = get_unix_time_now();
        self.arenas
            .retain(|_arena_id, arena| !arena.other_server || now < arena.date_put + 5000);
    }

    /// Server (re)starts arena when the executable is run.
    pub fn start_arena(
        &mut self,
        game_id: GameId,
        region_id: RegionId,
        rules: Option<RulesDto>,
        saved_arena_id: Option<ArenaId>,
        server_id: Option<ServerId>,
    ) -> ArenaId {
        let mut result: Option<ArenaId> = None;
        if let Some(arena_id) = saved_arena_id {
            if let Some(arena) = self.arenas.get_mut(&arena_id) {
                if arena.other_server {
                    // Get rid of other_server arena because it doesn't have complete parameters.
                    self.arenas.remove(&arena_id);
                } else {
                    arena.date_start = get_unix_time_now();
                    arena.other_server = false;
                    result = Some(arena_id);
                }
            }
        }

        if result.is_none() {
            if let Some(server_id) = server_id {
                // This ensures consistent arena IDs even if arena IDs are not
                // persisted by the game server.  Assume there is one arena per
                // server, and fewer than 1,000 servers.  In the unlikely event
                // this doesn't find an available arena ID, assume that the arena
                // ID generated in the subsequent clause is likely to be over 1,000.
                let n = NonZeroU32::new(server_id.0.get() as u32 + 1000).unwrap();
                let arena_id = ArenaId(n);
                if let Entry::Vacant(e) = self.arenas.entry(arena_id) {
                    e.insert(Arena::new(
                        game_id,
                        region_id,
                        rules.unwrap_or_default(),
                        Some(server_id),
                    ));
                    result = Some(arena_id);
                }
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

    /// Server reports it has stopped arena.
    pub fn stop_arena(&mut self, arena_id: ArenaId) {
        debug!("stop_arena(arena={:?})", arena_id);
        if let Some(arena) = Arena::get_mut(&mut self.arenas, arena_id) {
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
