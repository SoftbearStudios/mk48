// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::chat::ChatHistory;
use crate::generate_id::{generate_id, generate_id_64};
use crate::notify_set::NotifySet;
use crate::repo::Repo;
use core_protocol::dto::MessageDto;
use core_protocol::id::*;
use core_protocol::name::{Location, PlayerAlias, Referer, ServerAddr};
use core_protocol::UnixTime;
use core_protocol::*;
use log::{debug, info, trace};
use rustrict::CensorIter;
use std::collections::hash_map::{Entry, HashMap};
use std::collections::HashSet;
use std::rc::Rc;

pub struct Play {
    pub date_created: UnixTime,
    pub date_join: Option<UnixTime>,
    pub date_stop: Option<UnixTime>,
    pub score: Option<u32>, // e.g. 1234
    pub team_captain: bool,
    pub team_id: Option<TeamId>,
    pub whisper_joins: NotifySet<TeamId>,     // For joiner.
    pub whisper_joiners: NotifySet<PlayerId>, // For captain.
}

#[allow(dead_code)]
pub struct Session {
    pub alias: PlayerAlias,
    pub arena_id: ArenaId,
    pub bot: bool,
    // Whether this session is the player's first.
    pub chat_history: ChatHistory,
    pub date_created: UnixTime,
    pub date_drop: Option<UnixTime>,
    /// When last called create_session.
    pub date_renewed: UnixTime,
    pub date_terminated: Option<UnixTime>,
    pub game_id: GameId,
    pub language: LanguageId,
    pub location: Option<Location>, // e.g. (001, 215, 912)
    pub inbox: Vec<Rc<MessageDto>>, // For recipient (even if NOT playing).
    pub muted: HashSet<PlayerId>,
    pub live: bool,
    pub player_id: PlayerId,
    pub plays: Vec<Play>,
    pub previous_id: Option<SessionId>,
    pub referer: Option<Referer>,
    pub region_id: RegionId,
    pub server_addr: ServerAddr,
    pub whisper_muted: NotifySet<PlayerId>, // For muter (even if NOT playing).
}

impl Play {
    pub fn new() -> Self {
        let date_created = get_unix_time_now();
        Self {
            date_created,
            date_join: None,
            date_stop: None,
            score: None,
            team_captain: false,
            team_id: None,
            whisper_joins: NotifySet::new(),
            whisper_joiners: NotifySet::new(),
        }
    }

    // Returns true if the player might be on the liveboard.
    pub fn exceeds_score(&self, min_score: u32) -> bool {
        let mut result = false;
        if let Some(score) = self.score {
            if score >= min_score {
                result = true;
            }
        }
        result
    }
}

impl Session {
    pub fn new(
        alias: PlayerAlias,
        arena_id: ArenaId,
        bot: bool,
        game_id: GameId,
        language: LanguageId,
        player_id: PlayerId,
        previous_id: Option<SessionId>,
        referer: Option<Referer>,
        region_id: RegionId,
        server_addr: ServerAddr,
    ) -> Self {
        let date_created = get_unix_time_now();
        Self {
            alias,
            arena_id,
            bot,
            chat_history: ChatHistory::new(),
            date_created,
            date_drop: None,
            date_renewed: date_created,
            date_terminated: None,
            game_id,
            inbox: Vec::new(),
            language,
            live: false,
            location: None,
            muted: HashSet::new(),
            referer,
            region_id,
            player_id,
            plays: Vec::new(),
            previous_id,
            server_addr,
            whisper_muted: NotifySet::new(),
        }
    }

    pub fn get_mut<'a>(
        sessions: &'a mut HashMap<SessionId, Session>,
        session_id: &'a SessionId,
    ) -> Option<&'a mut Session> {
        let mut result = None;
        if let Some(session) = sessions.get_mut(session_id) {
            if session.date_terminated.is_none() {
                result = Some(session);
            }
        }
        result
    }

    pub fn iter_mut(
        sessions: &mut HashMap<SessionId, Session>,
    ) -> impl Iterator<Item = (&SessionId, &mut Session)> {
        sessions
            .iter_mut()
            .filter_map(move |(session_id, session)| {
                if session.date_terminated.is_none() {
                    Some((session_id, session))
                } else {
                    None
                }
            })
    }

    /// Terminate a session and stop all of its plays.
    pub fn terminate_session(&mut self) -> bool {
        if self.date_terminated.is_none() {
            let now = Some(get_unix_time_now());
            self.date_terminated = now;
            self.live = false;
            if let Some(play) = self.plays.last_mut() {
                if play.date_stop.is_none() {
                    play.date_stop = now;
                }
            }
            true
        } else {
            false
        }
    }
}

impl Repo {
    /// Finds an active `SessionId` and its last `Play`, if they exist (mutable version).
    pub fn player_id_to_session_and_play_mut<'a, 'b>(
        players: &'a mut HashMap<PlayerId, SessionId>,
        sessions: &'b mut HashMap<SessionId, Session>,
        player_id: PlayerId,
    ) -> Option<(SessionId, &'b mut Play)> {
        if let Some(session_id) = players.get_mut(&player_id) {
            let session = sessions.get_mut(session_id).unwrap();
            if let Some(play) = session.plays.last_mut() {
                if session.player_id == player_id
                    && session.date_terminated.is_none()
                    && session.live
                {
                    return Some((session_id.clone(), play));
                }
            }
        }
        None
    }

    /// Finds the most recent `SessionId` (if one exists) for the specified `PlayerId`.
    pub fn player_id_to_name(&self, arena_id: ArenaId, player_id: PlayerId) -> Option<PlayerAlias> {
        if let Some(arena) = self.arenas.get(&arena_id) {
            if let Some(session_id) = self.players.get(&player_id) {
                if let Some(session) = arena.sessions.get(session_id) {
                    return Some(session.alias);
                }
            }
        }
        None
    }

    pub fn create_entity(
        players: &mut HashMap<PlayerId, SessionId>,
        session_id: SessionId,
    ) -> PlayerId {
        loop {
            let player_id = PlayerId(generate_id());
            if let Entry::Vacant(e) = players.entry(player_id) {
                e.insert(session_id);
                break player_id;
            }
        }
    }

    /// Creates a session or renews a saved session.  Assumes saved session put into cache if possible.
    pub fn create_session(
        &mut self,
        alias: Option<PlayerAlias>,
        game_id: GameId,
        // TODO: invitation_id: Option<InvitationId>,
        language_pref: Option<LanguageId>,
        referer: Option<Referer>,
        region_pref: Option<RegionId>,
        saved_session_tuple: Option<(ArenaId, SessionId)>,
    ) -> Option<(ArenaId, LanguageId, RegionId, SessionId, ServerAddr)> {
        // TODO: if invitation_id is present, then use invitation.arena_id, and join invitation.team_id.
        info!("entering create_session");
        let mut saved_player_id = None;
        if let Some((arena_id, session_id)) = saved_session_tuple {
            if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
                if let Some(session) = arena.sessions.get_mut(&session_id) {
                    saved_player_id = Some(session.player_id);
                    let mut terminate = false;
                    if arena.game_id != game_id {
                        terminate = true;
                    }
                    if !terminate && region_pref != None && arena.region_id != region_pref.unwrap()
                    {
                        terminate = true;
                    }
                    info!("create_session will terminate incompatible={}", terminate);
                    if terminate {
                        session.terminate_session();
                    } else {
                        // It is OK to change parameters like alias, language and referer.
                        if let Some(uncensored_alias) = alias {
                            let censored_text =
                                uncensored_alias.0.chars().censor().collect::<String>();
                            session.alias = PlayerAlias::new(&censored_text);
                        }
                        if let Some(language) = language_pref {
                            session.language = language;
                        }
                        if let Some(referer) = referer {
                            session.referer = Some(referer);
                        }

                        session.date_drop = None;
                        session.date_renewed = get_unix_time_now();
                        debug!("recreate_session() => session={:?}", session_id);
                        return Some((
                            arena_id,
                            session.language,
                            session.region_id,
                            session_id,
                            arena.server_addr.clone(),
                        ));
                    }
                }
            }
        }

        info!("session not recreated");

        // If not recreated...
        let region_id = region_pref.unwrap_or_default();
        let mut found: Option<(ArenaId, &mut Arena)> = None;
        for (arena_id, arena) in Arena::iter_mut(&mut self.arenas) {
            if arena.game_id == game_id && arena.region_id == region_id {
                found = Some((*arena_id, arena));
                break;
            }
        }

        if let Some((arena_id, arena)) = found {
            info!("found compatible arena");

            let effective_alias = if let Some(a) = alias {
                PlayerAlias::new(&a.0.chars().censor().collect::<String>())
            } else {
                PlayerAlias::new("Guest")
            };
            let language = language_pref.unwrap_or_default();
            loop {
                let previous_id = if let Some((_, session_id)) = saved_session_tuple {
                    Some(session_id)
                } else {
                    None
                };
                // Use the date so that a session_id from a prior day is guaranteed to be different.
                let session_id = SessionId(generate_id_64());
                if let Entry::Vacant(e) = arena.sessions.entry(session_id) {
                    let server_addr = arena.server_addr.clone();
                    let tuple = Some((
                        arena_id,
                        language,
                        region_id,
                        session_id,
                        server_addr.clone(),
                    ));
                    let player_id = if let Some(player_id) = saved_player_id {
                        self.players.insert(player_id, session_id);
                        player_id
                    } else {
                        Self::create_entity(&mut self.players, session_id)
                    };
                    debug!(
                        "create_session(alias={:?}) => session={:?}, player={:?}",
                        &effective_alias, session_id, player_id
                    );
                    let bot = false;
                    e.insert(Session::new(
                        effective_alias,
                        arena_id,
                        bot,
                        game_id,
                        language,
                        player_id,
                        previous_id,
                        referer,
                        region_id,
                        server_addr,
                    ));
                    return tuple;
                }
            }
        }

        info!("no session returned");

        None
    }

    // Server reports that client dropped web socket.
    pub fn drop_session(&mut self, arena_id: ArenaId, session_id: SessionId) {
        debug!(
            "drop_session(arena={:?}, session={:?})",
            arena_id, session_id
        );
        if let Some(arena) = self.arenas.get_mut(&arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, &session_id) {
                if session.date_drop.is_none() {
                    session.date_drop = Some(get_unix_time_now());
                    if let Some(play) = session.plays.last_mut() {
                        if play.date_stop.is_none() {
                            // This makes a player eligible to be removed via arena.broadcast_players.removed(session_id)
                            play.date_stop = Some(get_unix_time_now());
                        }
                    }
                }
            }
        }
    }

    /// Returns true if the (arena_id, session_id) is in the in-memory cache.
    pub fn is_session_in_cache(&mut self, session_tuple: Option<(ArenaId, SessionId)>) -> bool {
        if let Some((arena_id, session_id)) = session_tuple {
            if let Some(arena) = self.arenas.get_mut(&arena_id) {
                if arena.sessions.contains_key(&session_id) {
                    return true;
                }
            }
        }
        false
    }

    /// Iterates recently modified, non-bot sessions.
    pub fn iter_recently_modified_sessions(
        &mut self,
        period: UnixTime,
    ) -> impl Iterator<Item = (ArenaId, SessionId, &Session)> {
        let threshold = get_unix_time_now() - period;
        self.arenas.iter().flat_map(move |(arena_id, arena)| {
            arena
                .sessions
                .iter()
                .filter_map(move |(session_id, session)| {
                    if !session.bot
                        && (session.date_renewed >= threshold
                            || (session.date_terminated.is_some()
                                && session.date_terminated.unwrap() >= threshold))
                    {
                        Some((*arena_id, *session_id, session))
                    } else {
                        None
                    }
                })
        })
    }

    // Assume this is called every minute to prune live sessions.
    pub fn prune_sessions(&mut self) {
        let date_dead = get_unix_time_now() - Self::DYING_DURATION_MILLIS;
        for (_, arena) in Arena::iter_mut(&mut self.arenas) {
            for (session_id, session) in Session::iter_mut(&mut arena.sessions) {
                if !session.live {
                    continue;
                }
                let play = session.plays.last_mut().unwrap();
                if let Some(date_stop) = play.date_stop {
                    if date_stop < date_dead {
                        session.live = false;
                        if let Some(team_id) = play.team_id {
                            arena.confide_membership.insert(session.player_id, None);
                            arena.broadcast_players.removed(*session_id);
                            if play.team_captain {
                                if let Some(team) = arena.teams.get(&team_id) {
                                    for joiner in team.joiners.iter() {
                                        play.whisper_joiners.removed(*joiner);
                                    }
                                }
                            }
                        }
                        arena.broadcast_players.removed(*session_id);
                    }
                }
            } // for session
        } // for arena
    }

    /// Assume caller uses this method to populate cache with result of database query.
    pub fn put_session(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        session: Session,
    ) -> bool {
        if let Some(arena) = self.arenas.get_mut(&arena_id) {
            arena.sessions.insert(session_id, session);
            true
        } else {
            false
        }
    }

    // Server sets player's status (location and/or score).
    // The liveboard will be updated accordingly.
    pub fn set_status(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        location: Option<Location>,
        score: Option<u32>,
    ) {
        trace!("set_status(arena={:?}, session={:?})", arena_id, session_id);
        let mut liveboard_changed = false;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, &session_id) {
                if let Some(play) = session.plays.last_mut() {
                    if play.date_stop.is_none() {
                        if let Some(value) = location {
                            session.location = Some(value);
                            liveboard_changed = true;
                        }
                        if let Some(value) = score {
                            play.score = Some(value);
                            liveboard_changed = true;
                        }
                    }
                }
            }
        }
        if liveboard_changed {
            if let Some(arena) = self.arenas.get_mut(&arena_id) {
                arena.liveboard_changed = true;
            }
        }
    }

    // Server reports that player joined game.  Useful for reports.
    pub fn start_play(&mut self, arena_id: ArenaId, session_id: SessionId) -> Option<PlayerId> {
        let mut result = None;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, &session_id) {
                if !session.bot {
                    debug!("start_play(arena={:?}, session={:?})", arena_id, session_id);
                }
                let mut new_play = Play::new();
                if session.live {
                    // Live sessions inherit previous team and captaincy.
                    let last_play = session.plays.last().unwrap();
                    new_play.team_id = last_play.team_id;
                    new_play.team_captain = last_play.team_captain;
                } else {
                    // Other sessions are added to the roster as they become live.
                    session.live = true;
                    arena.broadcast_players.added(session_id);
                }
                session.plays.push(new_play);
                result = Some(session.player_id);
            }
        }
        if result == None {
            debug!(
                "start_play(arena={:?}, session={:?}) failed",
                arena_id, session_id
            );
        }
        return result;
    }

    // Server reports that player left game.  Nevertheless session remains live for a while.
    pub fn stop_play(&mut self, arena_id: ArenaId, session_id: SessionId) {
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, &session_id) {
                if !session.bot {
                    debug!("stop_play(arena={:?}, session={:?})", arena_id, session_id);
                }
                if let Some(play) = session.plays.last_mut() {
                    if play.date_stop.is_none() {
                        play.date_stop = Some(get_unix_time_now());
                    }
                    if play.exceeds_score(arena.liveboard_min_score) {
                        // Even if session remains live, remove from liveboard when play stops.
                        arena.liveboard_changed = true;
                    }
                    // No need to touch arena.broadcast_players because the session remains live.
                }
            }
        }
    }

    // Client terminates old session due upon creating a new session.
    pub fn terminate_session(&mut self, arena_id: ArenaId, session_id: SessionId) {
        debug!(
            "terminate_session(arena={:?}, session={:?})",
            arena_id, session_id
        );
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, &session_id) {
                let was_live = session.live;
                if session.terminate_session() && was_live {
                    arena.broadcast_players.removed(session_id);
                    // See also: prune_teams()
                }
            }
        }
    }

    /// Server validates client's session.  Even a terminated session is valid.
    pub fn validate_session(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
    ) -> Option<(u32, PlayerId, u32)> {
        debug!(
            "validate_session(arena={:?}, session={:?})",
            arena_id, session_id
        );
        let mut result = None;
        if let Some(arena) = self.arenas.get(&arena_id) {
            if let Some(session) = arena.sessions.get(&session_id) {
                result = Some((0, session.player_id, 0));
                if session.live {
                    if let Some(play) = session.plays.last() {
                        if let Some(score) = play.score {
                            let elapsed = if let Some(date_stop) = play.date_stop {
                                get_unix_time_now() - date_stop
                            } else {
                                0
                            };
                            result = Some((elapsed as u32, session.player_id, score));
                        }
                    }
                }
            }
        }
        result
    }
}
