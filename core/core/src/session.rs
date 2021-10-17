// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::chat::ChatHistory;
use crate::generate_id::{generate_id, generate_id_64};
use crate::invitation::Invitation;
use crate::notify_set::NotifySet;
use crate::repo::Repo;
use core_protocol::dto::{InvitationDto, MessageDto};
use core_protocol::id::*;
use core_protocol::name::{trim_spaces, Location, PlayerAlias, Referrer};
use core_protocol::UnixTime;
use core_protocol::*;
use log::{debug, info, trace, warn};
use rustrict::CensorIter;
use std::collections::hash_map::{Entry, HashMap};
use std::collections::HashSet;
use std::rc::Rc;

pub struct Play {
    pub date_created: UnixTime,
    pub date_join: Option<UnixTime>,
    pub date_stop: Option<UnixTime>,
    pub invited: bool,
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
    pub chat_history: ChatHistory,
    pub date_created: UnixTime,
    pub date_drop: Option<UnixTime>,
    /// When last called create_session.
    pub date_renewed: UnixTime,
    pub date_terminated: Option<UnixTime>,
    pub game_id: GameId,
    /// e.g. (001, 215, 912)
    pub location: Option<Location>,
    // For recipient (even if NOT playing).
    pub inbox: Vec<Rc<MessageDto>>,
    /// Inbound invitation to consume (accept) when starting the next play.
    pub invitation: Option<Invitation>,
    /// Previously created outbound invitation id (useful to prevent creating multiple).
    pub invitation_id: Option<InvitationId>,
    pub muted: HashSet<PlayerId>,
    pub live: bool,
    pub player_id: PlayerId,
    pub plays: Vec<Play>,
    pub previous_id: Option<SessionId>,
    pub previous_plays: u32,
    pub referrer: Option<Referrer>,
    pub server_id: Option<ServerId>,
    pub user_agent_id: Option<UserAgentId>,
    pub whisper_muted: NotifySet<PlayerId>, // For muter (even if NOT playing).
}

impl Play {
    pub fn new() -> Self {
        let date_created = get_unix_time_now();
        Self {
            date_created,
            date_join: None,
            date_stop: None,
            invited: false,
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
        player_id: PlayerId,
        previous_id: Option<SessionId>,
        referrer: Option<Referrer>,
        server_id: Option<ServerId>,
        user_agent_id: Option<UserAgentId>,
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
            invitation: None,
            invitation_id: None,
            live: false,
            location: None,
            muted: HashSet::new(),
            referrer,
            player_id,
            plays: Vec::new(),
            previous_id,
            previous_plays: 0,
            server_id,
            user_agent_id,
            whisper_muted: NotifySet::new(),
        }
    }

    pub fn get_mut(
        sessions: &mut HashMap<SessionId, Session>,
        session_id: SessionId,
    ) -> Option<&mut Session> {
        let mut result = None;
        if let Some(session) = sessions.get_mut(&session_id) {
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

    /// Creates a session, or renews a saved session, and returns `session_id`
    /// and related values.  If the `invitation_id` or preferences is incompatible
    /// with all available arenas on the server, then no session is created an `None`
    /// is returned.  Assumes saved session put into cache if possible.
    pub fn create_session(
        &mut self,
        game_id: GameId,
        invitation_id: Option<InvitationId>,
        referrer: Option<Referrer>,
        saved_session_tuple: Option<(ArenaId, SessionId)>,
        user_agent_id: Option<UserAgentId>,
    ) -> Option<(ArenaId, SessionId, Option<ServerId>)> {
        info!(
            "create_session(game_id={:?}, invitation_id={:?}, user_agent_id={:?})",
            game_id, invitation_id, user_agent_id
        );

        if let Some(user_agent_id) = user_agent_id {
            if user_agent_id == UserAgentId::Spider {
                return None;
            }
        }

        let maybe_invitation = if let Some(invitation_id) = invitation_id {
            self.invitations.get(&invitation_id)
        } else {
            None
        };

        if invitation_id.is_some() {
            debug!("found invitation: {:?}", maybe_invitation);
        }

        let mut saved_player_id = None;
        if let Some((arena_id, session_id)) = saved_session_tuple {
            if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
                if let Some(session) = arena.sessions.get_mut(&session_id) {
                    saved_player_id = Some(session.player_id);
                    let mut terminate = false;
                    if arena.game_id != game_id {
                        terminate = true;
                    }
                    if !terminate {
                        if let Some(invitation) = maybe_invitation {
                            if invitation.arena_id != arena_id {
                                terminate = true;
                            }
                        }
                    }
                    if session.date_terminated.is_some() {
                        info!("session already terminated {:?}", session_id)
                    } else if terminate {
                        info!("terminating incompatible session {:?}", session_id);
                        session.terminate_session();
                    } else {
                        info!("renewing compatible session {:?}", session_id);
                        // It is OK to change parameters like referrer and user agent.
                        if let Some(invitation) = maybe_invitation {
                            // Will be consumed in start_play().
                            session.invitation = Some(invitation.clone());
                        }
                        if let Some(referrer) = referrer {
                            session.referrer = Some(referrer);
                        }
                        if user_agent_id.is_some() {
                            session.user_agent_id = user_agent_id;
                        }

                        session.date_drop = None;
                        session.date_renewed = get_unix_time_now();
                        return Some((arena_id, session_id, arena.server_id));
                    }
                }
            }
        }

        // If not renewed...
        info!("session was not renewed");

        let mut found: Option<(ArenaId, &mut Arena)> = None;
        for (arena_id, arena) in Arena::iter_mut(&mut self.arenas) {
            if arena.game_id != game_id {
                continue;
            }
            if let Some(invitation) = maybe_invitation {
                if invitation.arena_id != *arena_id {
                    continue;
                }
            }
            found = Some((*arena_id, arena));
            break;
        }

        if let Some((arena_id, arena)) = found {
            info!("found compatible arena");

            let guest_alias = PlayerAlias::new("Guest");
            loop {
                let previous_id = if let Some((_, session_id)) = saved_session_tuple {
                    Some(session_id)
                } else {
                    None
                };
                // Use the date so that a session_id from a prior day is guaranteed to be different.
                let session_id = SessionId(generate_id_64());
                if let Entry::Vacant(e) = arena.sessions.entry(session_id) {
                    let server_id = arena.server_id;
                    let tuple = Some((arena_id, session_id, server_id));
                    let player_id = if let Some(player_id) = saved_player_id {
                        self.players.insert(player_id, session_id);
                        player_id
                    } else {
                        Self::create_entity(&mut self.players, session_id)
                    };
                    debug!(
                        "create_session(alias={:?}) => session={:?}, player={:?}",
                        &guest_alias, session_id, player_id
                    );
                    let bot = false;
                    let mut session = Session::new(
                        guest_alias,
                        arena_id,
                        bot,
                        game_id,
                        player_id,
                        previous_id,
                        referrer,
                        server_id,
                        user_agent_id,
                    );
                    if let Some(invitation) = maybe_invitation {
                        session.invitation = Some(invitation.clone());
                    }
                    e.insert(session);
                    return tuple;
                }
            }
        }

        warn!(
            "could not create session for game_id={:?}, invitation_id={:?}",
            game_id, invitation_id
        );

        None
    }

    // Server reports that client dropped web socket.
    pub fn drop_session(&mut self, arena_id: ArenaId, session_id: SessionId) {
        debug!(
            "drop_session(arena={:?}, session={:?})",
            arena_id, session_id
        );
        if let Some(arena) = self.arenas.get_mut(&arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
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

    pub fn identify_session(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        uncensored_alias: PlayerAlias,
    ) -> bool {
        info!(
            "identify_session(arena_id={:?}, session_id={:?}, uncensored_alias={:?})",
            arena_id, session_id, uncensored_alias,
        );

        let mut identified = false;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                let now = get_unix_time_now();
                let mut prohibited = false;
                if session.live {
                    if let Some(play) = session.plays.last() {
                        if play.date_stop.is_none() {
                            let elapsed_millis = now.saturating_sub(play.date_created);
                            if elapsed_millis > 10000 {
                                // Prohibit alias changes after play starts lest it corrupt leaderboard.
                                prohibited = true;
                            }
                        }
                    }
                }
                if !prohibited {
                    let censored_text = uncensored_alias.0.chars().censor().collect::<String>();
                    let trimmed_text = trim_spaces(&censored_text);
                    if trimmed_text.len() > 0 {
                        let censored_alias = PlayerAlias::new(trimmed_text);
                        if session.alias != censored_alias {
                            session.alias = censored_alias;
                            session.date_renewed = now; // Persist alias.
                            if session.live {
                                // Tell clients about name change.
                                arena.broadcast_players.added(session_id);
                            }
                            identified = true;
                        }
                    }
                }
            }
        }

        if !identified {
            warn!(
                "identify_session(arena_id={:?}, session_id={:?}, uncensored_alias={:?}) failed",
                arena_id, session_id, uncensored_alias,
            );
        }

        identified
    }

    /// Returns true if the (arena_id, session_id) is in the in-memory cache.
    pub fn is_session_in_cache(&self, session_tuple: Option<(ArenaId, SessionId)>) -> bool {
        if let Some((arena_id, session_id)) = session_tuple {
            if let Some(arena) = self.arenas.get(&arena_id) {
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
        let now = get_unix_time_now();
        let date_dead = now - Self::DYING_DURATION_MILLIS;
        const TWENTY_FOUR_HOURS_IN_MILLIS: u64 = 24 * 60 * 60 * 1000;

        for (_, arena) in Arena::iter_mut(&mut self.arenas) {
            let mut removable = vec![];
            for (session_id, session) in Session::iter_mut(&mut arena.sessions) {
                if session.live {
                    // Prune live sessions.
                    let play = session.plays.last_mut().unwrap();
                    if let Some(date_stop) = play.date_stop {
                        if date_stop < date_dead {
                            session.live = false;
                            if let Some(team_id) = play.team_id {
                                arena.confide_membership.insert(session.player_id, None);
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
                } else if session.date_terminated.is_none() {
                    // Terminate non-live sessions.
                    let elapsed_millis = now.saturating_sub(session.date_renewed);
                    if elapsed_millis > TWENTY_FOUR_HOURS_IN_MILLIS {
                        session.date_terminated = Some(now);
                    }
                } else if let Some(date_terminated) = session.date_terminated {
                    // Remove terminated sessions.
                    let elapsed_millis = now.saturating_sub(date_terminated);
                    if elapsed_millis > TWENTY_FOUR_HOURS_IN_MILLIS {
                        removable.push(*session_id);
                    }
                }
            } // for session

            for session_id in removable {
                arena.sessions.remove(&session_id);
            }
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
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
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
        // Get this session's invitation if any.
        let invitation = self
            .arenas
            .get(&arena_id)
            .and_then(|a| a.sessions.get(&session_id))
            .and_then(|s| s.invitation.clone());
        let invited = invitation.is_some();

        // Get the team_id that can be joined with that invitation, if any.
        // TODO: Check if the team has space.
        let players = &mut self.players;
        let arenas = &mut self.arenas;
        let invitation_team_id = invitation
            .and_then(|inv| {
                arenas
                    .get_mut(&inv.arena_id)
                    .zip(players.get(&inv.player_id))
            })
            .and_then(|(a, &s)| a.team_of_captain(s))
            .map(|(team_id, _)| team_id);

        let mut result = None;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                if !session.bot {
                    debug!("start_play(arena={:?}, session={:?})", arena_id, session_id);
                }
                let mut new_play = Play::new();
                new_play.score = arena.rules.default_score;
                if new_play.exceeds_score(arena.liveboard_min_score) {
                    arena.liveboard_changed = true;
                }
                if invited {
                    new_play.invited = true;
                    session.invitation = None; // Once used, invitation is cleared.
                }
                if session.live {
                    // Live sessions inherit previous team and captaincy.
                    let last_play = session.plays.last().unwrap();
                    new_play.team_id = last_play.team_id;
                    new_play.team_captain = last_play.team_captain;
                } else {
                    // Other sessions are added to the roster as they become live.
                    session.live = true;
                    new_play.team_id = invitation_team_id;
                }

                // Player is either new or possibly changed their name. Also, game server may have
                // expired player already.
                arena.broadcast_players.added(session_id);
                arena
                    .confide_membership
                    .insert(session.player_id, new_play.team_id);

                session.plays.push(new_play);
                result = Some(session.player_id);
            }
        }
        if result == None {
            warn!(
                "start_play(arena={:?}, session={:?}) failed",
                arena_id, session_id
            );
        }
        return result;
    }

    // Server reports that player left game.  Nevertheless session remains live for a while.
    pub fn stop_play(&mut self, arena_id: ArenaId, session_id: SessionId) {
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
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
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
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
    ) -> Option<(u32, Option<InvitationDto>, PlayerId, u32)> {
        debug!(
            "validate_session(arena={:?}, session={:?})",
            arena_id, session_id
        );
        let mut result = None;
        if let Some(arena) = self.arenas.get(&arena_id) {
            if let Some(session) = arena.sessions.get(&session_id) {
                let invitation =
                    session
                        .invitation
                        .as_ref()
                        .map(|Invitation { player_id, .. }| InvitationDto {
                            player_id: *player_id,
                        });
                result = Some((0, invitation.clone(), session.player_id, 0));
                if session.live {
                    if let Some(play) = session.plays.last() {
                        if let Some(score) = play.score {
                            let elapsed = if let Some(date_stop) = play.date_stop {
                                get_unix_time_now() - date_stop
                            } else {
                                0
                            };
                            result = Some((elapsed as u32, invitation, session.player_id, score));
                        }
                    }
                }
            }
        }

        if result.is_none() {
            warn!(
                "validate_session(arena={:?}, session={:?}) failed",
                arena_id, session_id
            );
        }

        result
    }
}
