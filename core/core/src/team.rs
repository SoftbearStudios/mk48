// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::generate_id::generate_id;
use crate::repo::Repo;
use crate::session::Session;
use core_protocol::get_unix_time_now;
use core_protocol::id::*;
use core_protocol::name::*;
use log::{debug, error, warn};
use rustrict::CensorIter;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

pub struct Team {
    pub team_name: TeamName,
    pub joiners: HashSet<PlayerId>,
}

impl Repo {
    /// Client accepts another player into its team.
    pub fn accept_player(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        player_id: PlayerId,
    ) -> bool {
        debug!(
            "accept_player(arena={:?}, session={:?}, player={:?})",
            arena_id, session_id, player_id
        );
        let mut accepted = false;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            let mut captain_team_id: Option<TeamId> = None;
            if let Some((team_id, _)) = arena.team_of_captain(session_id) {
                captain_team_id = Some(team_id);
            }

            if let Some(team_id) = captain_team_id {
                let member_count = arena
                    .sessions
                    .iter()
                    .filter(|(_, s)| {
                        s.live
                            && s.plays
                                .last()
                                .map(|p| p.team_id == Some(team_id))
                                .unwrap_or(false)
                    })
                    .count();

                if member_count as u32 >= arena.rules.team_size_max {
                    return false;
                }

                if let Some((session_id, session)) =
                    Self::player_id_to_session_mut(&self.players, &mut arena.sessions, player_id)
                {
                    if let Some(team) = arena.teams.get_mut(&team_id) {
                        let play = session.plays.last_mut().unwrap();

                        // Don't allow arbitrary conscription!
                        if team.joiners.contains(&player_id) && play.team_id.is_none() {
                            play.team_id = Some(team_id);
                            play.date_join = Some(get_unix_time_now());
                            accepted = true;
                            if play.exceeds_score(arena.liveboard_min_score) {
                                arena.liveboard_changed = true;
                            }
                            arena.broadcast_players.added(session_id); // Team membership is on roster.
                            arena.confide_membership.insert(player_id, play.team_id);
                        }
                    }
                }

                if accepted {
                    // Purge all related joiners/joins, including on the team the joiner was just accepted into.
                    for (&other_team_id, team) in arena.teams.iter_mut() {
                        if team.joiners.remove(&player_id) {
                            if let Some(captain_session_id) =
                                Arena::static_captain_of_team(&arena.sessions, other_team_id)
                            {
                                if let Some(captain_session) =
                                    Session::get_mut(&mut arena.sessions, captain_session_id)
                                {
                                    captain_session.whisper_joiners.removed(player_id);
                                }
                            }

                            if let Some((_, session)) = Self::player_id_to_session_mut(
                                &self.players,
                                &mut arena.sessions,
                                player_id,
                            ) {
                                session.whisper_joins.removed(other_team_id);
                            }
                        }
                    }
                }
            }
        }

        if !accepted {
            warn!(
                "accept_player(arena={:?}, session={:?}, player={:?}) failed",
                arena_id, session_id, player_id
            );
        }

        accepted
    }

    // Client assigns another player as team captain.
    pub fn assign_captain(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        player_id: PlayerId,
    ) -> bool {
        debug!(
            "assign_captain(arena={:?}, session={:?}, player={:?})",
            arena_id, session_id, player_id
        );
        let mut assigned = false;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some((captain_team_id, _)) = arena.team_of_captain(session_id) {
                if let Some((_, session)) =
                    Self::player_id_to_session_mut(&self.players, &mut arena.sessions, player_id)
                {
                    let play = session.plays.last_mut().unwrap();
                    if let Some(team_id) = play.team_id {
                        if team_id == captain_team_id {
                            play.team_captain = true;
                            if let Some(team) = arena.teams.get(&team_id) {
                                for joiner in team.joiners.iter() {
                                    session.whisper_joiners.added(*joiner);
                                }
                            }
                            assigned = true;
                        }
                    }
                }

                if assigned {
                    // Un-captain the old captain if the new captain was assigned.
                    let session = arena.sessions.get_mut(&session_id).unwrap();
                    let play = session.plays.last_mut().unwrap();
                    play.team_captain = false;
                    if let Some(team) = arena.teams.get(&captain_team_id) {
                        for joiner in team.joiners.iter() {
                            session.whisper_joiners.removed(*joiner);
                        }
                    }
                }
            }
        }

        if !assigned {
            warn!(
                "assign_captain(arena={:?}, session={:?}, player={:?}) failed",
                arena_id, session_id, player_id
            );
        }

        assigned
    }

    // Client creates a new team.
    pub fn create_team(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        team_name: TeamName,
    ) -> Result<TeamId, &'static str> {
        debug!(
            "create_team(arena={:?}, session={:?}, team_name={:?})",
            arena_id, session_id, team_name
        );

        let censored_text = team_name.0.chars().censor().collect::<String>();
        let trimmed_text = trim_spaces(&censored_text);
        let censored_team_name = TeamName::new(trimmed_text);

        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                if let Some(play) = session.plays.last_mut() {
                    if !session.live {
                        return Err("session not live");
                    }
                    if censored_team_name.0.is_empty() {
                        return Err("empty team name");
                    }
                    if play.date_stop.is_some() {
                        return Err("play stopped");
                    }
                    if play.team_id.is_some() {
                        return Err("already in team");
                    }
                    for Team { team_name, .. } in arena.teams.values() {
                        if team_name == &censored_team_name {
                            return Err("team name in use");
                        }
                    }
                    let team_id = loop {
                        let team_id = TeamId(generate_id());
                        if let Entry::Vacant(e) = arena.teams.entry(team_id) {
                            e.insert(Team {
                                team_name: censored_team_name,
                                joiners: HashSet::new(),
                            });
                            arena.broadcast_teams.added(team_id);
                            break team_id;
                        }
                    };
                    if play.exceeds_score(arena.liveboard_min_score) {
                        arena.liveboard_changed = true;
                    }
                    play.team_captain = true;
                    play.team_id = Some(team_id);
                    arena.broadcast_players.added(session_id); // Team membership is on roster.
                    arena
                        .confide_membership
                        .insert(session.player_id, play.team_id);

                    let player_id = session.player_id;
                    for (team_id, team) in arena.teams.iter_mut() {
                        if team.joiners.remove(&player_id) {
                            if let Some(joiner_session) = arena.sessions.get_mut(&session_id) {
                                joiner_session.whisper_joins.removed(*team_id);
                            }
                            if let Some(captain_session_id) =
                                Arena::static_captain_of_team(&arena.sessions, *team_id)
                            {
                                if let Some(captain_session) =
                                    arena.sessions.get_mut(&captain_session_id)
                                {
                                    captain_session.whisper_joiners.removed(player_id);
                                }
                            }
                        }
                    }

                    Ok(team_id)
                } else {
                    Err("not playing")
                }
            } else {
                Err("invalid session_id")
            }
        } else {
            Err("invalid arena_id")
        }
    }

    // Client removes another player from its team.
    pub fn kick_player(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        player_id: PlayerId,
    ) -> bool {
        debug!(
            "kick_player(arena={:?}, session={:?}, player={:?})",
            arena_id, session_id, player_id
        );
        let mut removed = false;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some((captain_team_id, _)) = arena.team_of_captain(session_id) {
                if let Some((session_id, play)) = Self::player_id_to_session_and_play_mut(
                    &self.players,
                    &mut arena.sessions,
                    player_id,
                ) {
                    if let Some(team_id) = play.team_id {
                        if team_id == captain_team_id {
                            play.team_id = None;
                            removed = true;
                            arena.broadcast_players.added(session_id); // Team membership is on roster.
                            arena.confide_membership.insert(player_id, None);
                        }
                    }
                }
            }
        }

        if !removed {
            warn!(
                "kick_player(arena={:?}, session={:?}, player={:?}) failed",
                arena_id, session_id, player_id
            );
        }

        removed
    }

    // Assume this is called every minute to prune teams, promote captains, etc.
    pub fn prune_teams(&mut self) {
        for (_, arena) in Arena::iter_mut(&mut self.arenas) {
            struct Tally {
                captain: bool,
                members: HashSet<SessionId>,
            }

            // Step 1: start with the list of teams.
            let mut tallies: HashMap<TeamId, Tally> = HashMap::new();
            for (team_id, _) in arena.teams.iter() {
                tallies.insert(
                    *team_id,
                    Tally {
                        captain: false,
                        members: HashSet::new(),
                    },
                );
            }

            // Step 2: tally how many players there are in each team.
            for (session_id, session) in arena.sessions.iter() {
                if !session.live {
                    continue;
                }
                if let Some(play) = session.plays.last() {
                    if let Some(team_id) = play.team_id {
                        if let Some(tally) = tallies.get_mut(&team_id) {
                            if play.team_captain {
                                tally.captain = true;
                            }
                            tally.members.insert(*session_id);
                        }
                    }
                }
            } // for (session_id, session)

            // Step 3: prune empty teams and promote captains.
            for (team_id, tally) in tallies.iter_mut() {
                if tally.captain {
                    continue;
                }
                if tally.members.is_empty() {
                    debug!("pruning abandoned team {:?}", team_id);
                    arena.broadcast_teams.removed(*team_id);
                    if let Some(team) = arena.teams.get(team_id) {
                        for &joiner in team.joiners.iter() {
                            if let Some((_, session)) = Self::player_id_to_session_mut(
                                &self.players,
                                &mut arena.sessions,
                                joiner,
                            ) {
                                session.whisper_joins.removed(*team_id);
                            }
                        }
                        arena.teams.remove(team_id);
                    }
                    continue;
                } else {
                    debug!("promote new captain for team {:?}", team_id);
                    // Pick a captain according by seniority.
                    let mut members: Vec<SessionId> = tally.members.iter().cloned().collect();
                    members.sort_by(|session_id_a, session_id_b| {
                        let session_a = arena.sessions.get(session_id_a).unwrap();
                        let play_a = session_a.plays.last().unwrap();
                        // This panicked in the past when forgot to set date_join. Never lose entire
                        // server over this again!
                        let join_a = play_a.date_join.unwrap_or(0);
                        let session_b = arena.sessions.get(session_id_b).unwrap();
                        let play_b = session_b.plays.last().unwrap();
                        let join_b = play_b.date_join.unwrap_or(0);

                        join_a.cmp(&join_b)
                    });
                    let captain_session_id = members.first().unwrap();
                    let session = arena.sessions.get_mut(captain_session_id).unwrap();
                    let play = session.plays.last_mut().unwrap();
                    play.team_captain = true;
                    if let Some(team) = arena.teams.get(team_id) {
                        for joiner in team.joiners.iter() {
                            session.whisper_joiners.added(*joiner);
                        }
                    }
                    arena.broadcast_players.added(*captain_session_id); // Team membership is on roster.
                }
            } // for (team_id, tally)
        }
    }

    // Client wants to go it alone.
    pub fn quit_team(&mut self, arena_id: ArenaId, session_id: SessionId) -> bool {
        debug!("quit_team(arena={:?}, session={:?})", arena_id, session_id);
        let mut quit = false;
        let mut captain = false;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                if let Some(play) = session.plays.last_mut() {
                    if play.team_captain {
                        if let Some(team_id) = play.team_id {
                            if let Some(team) = arena.teams.get(&team_id) {
                                for joiner in team.joiners.iter() {
                                    session.whisper_joiners.removed(*joiner);
                                }
                            }
                        }
                        captain = true;
                        play.team_captain = false;
                    }

                    play.team_id = None;
                    arena.broadcast_players.added(session_id); // Team membership is on roster.
                    arena.confide_membership.insert(session.player_id, None);
                    quit = true;
                }
            }
        }
        if captain {
            // If possible, promote somebody else immediately.
            self.prune_teams();
        }

        if !quit {
            warn!(
                "quit_team(arena={:?}, session={:?}) failed",
                arena_id, session_id
            );
        }

        quit
    }

    // Client rejects another player's join request.
    pub fn reject_player(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        player_id: PlayerId,
    ) -> bool {
        debug!(
            "reject_player(arena={:?}, session={:?}, player={:?})",
            arena_id, session_id, player_id
        );
        let mut rejected = false;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            let mut captain_team_id: Option<TeamId> = None;
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                if let Some(play) = session.plays.last_mut() {
                    if play.team_captain {
                        session.whisper_joiners.removed(player_id);
                        captain_team_id = play.team_id;
                    }
                }
            }

            if let Some(team_id) = captain_team_id {
                if let Some((_, session)) =
                    Self::player_id_to_session_mut(&self.players, &mut arena.sessions, player_id)
                {
                    rejected = true;
                    session.whisper_joins.removed(team_id);
                    if let Some(team) = arena.teams.get_mut(&team_id) {
                        team.joiners.remove(&player_id);
                    }
                }
            }
        }

        if !rejected {
            warn!(
                "reject_player(arena={:?}, session={:?}, player={:?}) failed",
                arena_id, session_id, player_id
            );
        }

        rejected
    }

    // Client no longer wants to go it alone.
    pub fn request_join(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        team_id: TeamId,
    ) -> bool {
        debug!(
            "request_join(arena={:?}, session={:?}, team={:?})",
            arena_id, session_id, team_id
        );
        let mut requested = false;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            let mut joiner_player_id: Option<PlayerId> = None;
            let captain_session_id = arena.captain_of_team(team_id);
            if captain_session_id.is_some() {
                if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                    if let Some(play) = session.plays.last() {
                        if play.team_id.is_none() {
                            session.whisper_joins.added(team_id);
                            joiner_player_id = Some(session.player_id);
                        }
                    }
                }
            }

            if let Some(player_id) = joiner_player_id {
                let session = arena
                    .sessions
                    .get_mut(&captain_session_id.unwrap())
                    .unwrap();
                session.whisper_joiners.added(player_id);
                requested = true;

                if let Some(team) = arena.teams.get_mut(&team_id) {
                    team.joiners.insert(player_id);
                } else {
                    error!("team gone in request join");
                }
            }
        }

        if !requested {
            warn!(
                "request_join(arena={:?}, session={:?}, team={:?})",
                arena_id, session_id, team_id
            );
        }

        requested
    }
}
