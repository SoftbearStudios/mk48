// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::core::DB_SESSION_TIMER_SECS;
use crate::generate_id::{generate_id, generate_id_64};
use crate::invitation::Invitation;
use crate::notify_set::NotifySet;
use crate::repo::Repo;
use core_protocol::dto::{InvitationDto, MessageDto};
use core_protocol::get_unix_time_now;
use core_protocol::id::*;
use core_protocol::name::{Location, PlayerAlias, Referrer};
use core_protocol::UnixTime;
use heapless::HistoryBuffer;
use log::{debug, info, trace, warn};
use std::collections::hash_map::{Entry, HashMap};
use std::collections::HashSet;
use std::rc::Rc;

pub struct Play {
    pub date_created: UnixTime,

    /// If player joined a team, this date/time is used calculate seniority.
    pub date_join: Option<UnixTime>,

    /// If play was stopped, this is when it happened.
    pub date_stop: Option<UnixTime>,

    /// True if play was started via an invitation from another player.
    pub invited: bool,

    /// True if this is the first play since session was created or renewed.
    pub renewed: bool,

    /// The most recent score sent by game server, if any.
    pub score: Option<u32>, // e.g. 1234

    /// True if player became captain of a team.
    pub team_captain: bool,

    /// If player joined team, this is the team ID.
    pub team_id: Option<TeamId>,
}

pub struct Session {
    /// The most recent alias (pseudonym) of the player that owns this session.
    pub alias: PlayerAlias,

    /// The ID of the arena that this session is connected to.
    pub arena_id: ArenaId,

    /// The chat context is used to filter profanity and detect toxicity.
    pub chat_context: rustrict::Context,

    /// When this session was created.
    pub date_created: UnixTime,

    /// If this session was ever dropped this is the most recent date/time it happened.
    pub date_drop: Option<UnixTime>,

    /// If this session has ancestors, this is when the oldest was created.  Used to calculate retention.
    pub date_previous: Option<UnixTime>,

    /// When last called create_session.
    pub date_renewed: UnixTime,

    /// If this session was terminated, this is when it happened.  A session is terminated
    /// after 24 hours of inactivity.  After which, a new session ID is required to play.
    pub date_terminated: Option<UnixTime>,

    /// Frames per second, if reported by the game client.
    pub fps: Option<f32>,

    // The ID of the game that this session is connected to.
    pub game_id: GameId,

    /// e.g. (001, 215, 912)
    pub location: Option<Location>,

    // The inbox of messages that the player who owns this session will receive.
    // Appended to even when NOT playing, so that the player has messages upon return.
    pub inbox: HistoryBuffer<Rc<MessageDto>, 10>,

    /// If player was invited, invitation to consume (accept) when starting next play.
    pub invitation: Option<Invitation>,

    /// If player created invitation, outbound invitation id (useful to prevent creating multiple).
    pub invitation_id: Option<InvitationId>,

    /// Other players that were muted by player that owns this session.
    pub muted: HashSet<PlayerId>,

    /// Prevent multiple abuse reports from same user.
    pub reported: HashSet<PlayerId>,

    /// Network latency/round trip time (millis).
    pub rtt: Option<u16>,

    /// Whether this session is live, meaning that player is now (or recently was) playing.
    pub live: bool,

    // The ID of the player that owns this session.
    pub player_id: PlayerId,

    // A transcript of the recent plays in this session, used to calculate metrics.
    pub plays: Vec<Play>,

    // TODO: combine all previous into tuple, e.g. previous_session: Option<(ArenaId, SessionId, u32, UnixTime)>,
    pub previous_id: Option<SessionId>,

    // The number of previous plays in this session beyond those already in "plays".
    pub previous_plays: u32,

    // If the player got to the game by clicking on a referrer page, this is the abbreviated referrer URL.
    pub referrer: Option<Referrer>,

    // The ID of the server that runs this session.
    pub server_id: Option<ServerId>,

    // If known, the user agent (browser type) of the browser used by the player who owns this session.
    pub user_agent_id: Option<UserAgentId>,

    /// For joiner.
    pub whisper_joins: NotifySet<TeamId>,

    /// For captain.
    pub whisper_joiners: NotifySet<PlayerId>,
}

impl Default for Play {
    fn default() -> Self {
        Self::new()
    }
}

impl Play {
    pub fn new() -> Self {
        let date_created = get_unix_time_now();
        Self {
            date_created,
            date_join: None,
            date_stop: None,
            invited: false,
            renewed: false,
            score: None,
            team_captain: false,
            team_id: None,
        }
    }

    // Returns true if the player might be on the liveboard.
    pub fn exceeds_score(&self, min_score: u32) -> bool {
        self.score.map(|s| s >= min_score).unwrap_or(false)
    }
}

impl Session {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        alias: PlayerAlias,
        arena_id: ArenaId,
        date_previous: Option<UnixTime>,
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
            chat_context: rustrict::Context::default(),
            date_created,
            date_drop: None,
            date_previous,
            date_renewed: date_created,
            date_terminated: None,
            fps: None,
            rtt: None,
            game_id,
            inbox: HistoryBuffer::new(),
            invitation: None,
            invitation_id: None,
            live: false,
            location: None,
            muted: HashSet::new(),
            reported: HashSet::new(),
            referrer,
            player_id,
            plays: Vec::new(),
            previous_id,
            previous_plays: 0,
            server_id,
            user_agent_id,
            whisper_joins: NotifySet::new(),
            whisper_joiners: NotifySet::new(),
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

    /// Terminate a session and stop all of its plays. Returns true if terminated now, false if already
    /// terminated.
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
        players: &'a HashMap<PlayerId, SessionId>,
        sessions: &'b mut HashMap<SessionId, Session>,
        player_id: PlayerId,
    ) -> Option<(SessionId, &'b mut Play)> {
        if let Some(session_id) = players.get(&player_id) {
            let session = sessions.get_mut(session_id).unwrap();
            if let Some(play) = session.plays.last_mut() {
                if session.player_id == player_id
                    && session.date_terminated.is_none()
                    && session.live
                {
                    return Some((*session_id, play));
                }
            }
        }
        None
    }

    /// Finds an active session.
    pub fn player_id_to_session_mut<'a, 'b>(
        players: &'a HashMap<PlayerId, SessionId>,
        sessions: &'b mut HashMap<SessionId, Session>,
        player_id: PlayerId,
    ) -> Option<(SessionId, &'b mut Session)> {
        if let Some(session_id) = players.get(&player_id) {
            let session = sessions.get_mut(session_id).unwrap();
            if session.player_id == player_id && session.date_terminated.is_none() && session.live {
                return Some((*session_id, session));
            }
        }
        None
    }

    /// Finds the most recent `SessionId` (if one exists) for the specified `PlayerId`.
    pub fn player_id_to_name(&self, arena_id: ArenaId, player_id: PlayerId) -> Option<PlayerAlias> {
        if let Some(arena) = Arena::get(&self.arenas, arena_id) {
            if let Some(session_id) = self.players.get(&player_id) {
                if let Some(session) = arena.sessions.get(session_id) {
                    return Some(session.alias);
                }
            }
        }
        None
    }

    /// Generates a [`PlayerId`] and associates it with a [`SessionId`].
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
    ) -> Option<(ArenaId, SessionId, PlayerId, Option<ServerId>)> {
        info!(
            "create_session(game_id={:?}, invitation_id={:?}, user_agent_id={:?})",
            game_id, invitation_id, user_agent_id
        );

        if user_agent_id == Some(UserAgentId::Spider) {
            return None;
        }

        let invitations = &self.invitations;
        let maybe_invitation = invitation_id.and_then(|id| invitations.get(&id));

        if invitation_id.is_some() {
            debug!("found invitation: {:?}", maybe_invitation);
        }

        let previous_tuple = {
            let mut result = None;
            if let Some((arena_id, session_id)) = saved_session_tuple {
                if let Some(arena) = self.arenas.get(&arena_id) {
                    let date_previous = arena
                        .sessions
                        .get(&session_id)
                        .map(|s| s.date_previous.unwrap_or(s.date_created));
                    result = Some((date_previous, session_id));
                }
            };
            result
        };

        debug!(
            "saved: {:?}, prev: {:?}",
            saved_session_tuple, previous_tuple
        );

        let mut saved_player_id = None;
        if let Some((arena_id, session_id)) = saved_session_tuple {
            if let Some(arena) = Arena::get_mut(&mut self.arenas, arena_id) {
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

                        if let Some(&old_session_id) = self.players.get(&session.player_id) {
                            if session_id != old_session_id {
                                println!(
                                    "FATAL ERROR: Session mismatch {:?} {:?}",
                                    session_id, old_session_id
                                );
                            }
                        }

                        self.players.insert(session.player_id, session_id);

                        session.date_drop = None;
                        session.date_renewed = get_unix_time_now();
                        return Some((arena_id, session_id, session.player_id, arena.server_id));
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
                if invitation.arena_id != arena_id {
                    continue;
                }
            }
            found = Some((arena_id, arena));
            break;
        }

        if let Some((arena_id, arena)) = found {
            info!("found compatible arena");

            let guest_alias = PlayerAlias::default();
            loop {
                let (date_previous, previous_id) = if let Some(previous_tuple) = previous_tuple {
                    (previous_tuple.0, Some(previous_tuple.1))
                } else {
                    (None, None)
                };
                // Use the date so that a session_id from a prior day is guaranteed to be different.
                let session_id = SessionId(generate_id_64());
                if let Entry::Vacant(e) = arena.sessions.entry(session_id) {
                    let server_id = arena.server_id;
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
                    let mut session = Session::new(
                        guest_alias,
                        arena_id,
                        date_previous,
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
                    return Some((arena_id, session_id, player_id, server_id));
                }
            }
        }

        warn!(
            "could not create session for game_id={:?}, invitation_id={:?}",
            game_id, invitation_id
        );

        None
    }

    /// Server reports that client dropped web socket.
    pub fn drop_session(&mut self, arena_id: ArenaId, session_id: SessionId) {
        debug!(
            "drop_session(arena={:?}, session={:?})",
            arena_id, session_id
        );
        if let Some(arena) = Arena::get_mut(&mut self.arenas, arena_id) {
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

    /// Client assigns alias to their session.
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
        if let Some(arena) = Arena::get_mut(&mut self.arenas, arena_id) {
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
                    let censored_alias = PlayerAlias::new(uncensored_alias.as_str());
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

    /// Iterates recently modified sessions.
    pub fn iter_recently_modified_sessions(
        &mut self,
        period: UnixTime,
    ) -> impl Iterator<Item = (ArenaId, SessionId, &Session)> {
        let threshold = get_unix_time_now() - period;
        Arena::iter(&self.arenas).flat_map(move |(arena_id, arena)| {
            arena
                .sessions
                .iter()
                .filter_map(move |(session_id, session)| {
                    if session.date_renewed >= threshold
                        || (session.date_terminated.is_some()
                            && session.date_terminated.unwrap() >= threshold)
                    {
                        Some((arena_id, *session_id, session))
                    } else {
                        None
                    }
                })
        })
    }

    /// Assume this is called every minute to prune live sessions.
    pub fn prune_sessions(&mut self) {
        let now = get_unix_time_now();
        let date_dead = now - Self::DYING_DURATION_MILLIS;
        const ONE_HOUR_IN_MILLIS: u64 = 60 * 60 * 1000;
        const TWO_DAYS_IN_MILLIS: u64 = 48 * ONE_HOUR_IN_MILLIS;

        // Only prune from valid arenas because Arena::prune_arenas() prunes all else.
        for (_, arena) in Arena::iter_mut(&mut self.arenas) {
            let mut removable = vec![];
            for (session_id, session) in arena.sessions.iter_mut() {
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
                                            session.whisper_joiners.removed(*joiner);
                                        }
                                    }
                                }
                            }
                            // TODO: This is likely helpful, but can't be here due to lifetime issues.
                            /* else {
                                for (&team_id, team) in arena.teams.iter_mut() {
                                    if team.joiners.remove(&session.player_id) {
                                        if let Some(captain_session_id) =
                                        Arena::static_captain_of_team(&arena.sessions, team_id)
                                        {
                                            if let Some(captain_session) =
                                            Session::get_mut(&mut arena.sessions, captain_session_id)
                                            {
                                                captain_session.whisper_joiners.removed(session.player_id);
                                            }
                                        }

                                        if let Some((_, session)) = Self::player_id_to_session_mut(
                                            &mut self.players,
                                            &mut arena.sessions,
                                            session.player_id,
                                        ) {
                                            session.whisper_joins.removed(team_id);
                                        }
                                    }
                                }
                            }*/
                            arena.broadcast_players.removed(*session_id);
                        }
                    }
                } else if session.date_terminated.is_none() {
                    // Terminate non-live sessions if they are at least 2 days old.
                    // (The first day is to guarantee IDs are unique, and is also useful
                    // for metrics; the second day is to speed up session creation.)
                    let elapsed_millis = now.saturating_sub(session.date_renewed);
                    if elapsed_millis > TWO_DAYS_IN_MILLIS {
                        session.date_terminated = Some(now);
                    }
                } else if let Some(date_terminated) = session.date_terminated {
                    // It is necessary to wait DB_SESSION_TIMER_SECS or more for
                    // terminated sessions to be saved.
                    debug_assert!(ONE_HOUR_IN_MILLIS > DB_SESSION_TIMER_SECS * 1000);
                    let elapsed_millis = now.saturating_sub(date_terminated);
                    if elapsed_millis > ONE_HOUR_IN_MILLIS {
                        removable.push((*session_id, session.player_id));
                    }
                }
            } // for session

            for (session_id, player_id) in removable {
                arena.sessions.remove(&session_id);
                if self.players.get(&player_id) == Some(&session_id) {
                    self.players.remove(&player_id);
                }
            }
        } // for arena
    }

    /// Assume caller uses this method to populate cache with result of database query.
    pub fn put_session(&mut self, arena_id: ArenaId, session_id: SessionId, session: Session) {
        let arena = self
            .arenas
            .entry(arena_id)
            .or_insert_with(|| Arena::create_other_server(session.game_id));
        arena.sessions.insert(session_id, session);
        arena.date_put = get_unix_time_now();
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
        if let Some(arena) = Arena::get_mut(&mut self.arenas, arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                if let Some(play) = session.plays.last_mut() {
                    if play.date_stop.is_none() {
                        if let Some(value) = location {
                            session.location = Some(value);
                        }
                        if let Some(value) = score {
                            play.score = Some(value);
                            if play.exceeds_score(arena.liveboard_min_score) {
                                arena.liveboard_changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Server reports that player joined game.  Useful for reports.
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
            .and_then(|(a, &s)| {
                let team_id = a.team_of_captain(s).map(|(id, _)| id);
                if let Some(team_id) = team_id {
                    if Self::team_full(&a.sessions, &a.rules, team_id) {
                        return None;
                    }
                }
                team_id
            });

        let mut result = None;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                debug!("start_play(arena={:?}, session={:?})", arena_id, session_id);
                let mut new_play = Play::new();
                new_play.renewed = session
                    .plays
                    .last()
                    .map(|p| p.date_created < session.date_renewed)
                    .unwrap_or(true);
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
                    let last_play = session.plays.last_mut().unwrap();
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
        result
    }

    /// Server reports that player left game.  Nevertheless session remains live for a while.
    pub fn stop_play(&mut self, arena_id: ArenaId, session_id: SessionId) {
        if let Some(arena) = Arena::get_mut(&mut self.arenas, arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                debug!("stop_play(arena={:?}, session={:?})", arena_id, session_id);
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

    /// Client terminates old session due upon creating a new session.
    pub fn terminate_session(&mut self, arena_id: ArenaId, session_id: SessionId) {
        debug!(
            "terminate_session(arena={:?}, session={:?})",
            arena_id, session_id
        );
        if let Some(arena) = Arena::get_mut(&mut self.arenas, arena_id) {
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
        if let Some(arena) = Arena::get(&self.arenas, arena_id) {
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
