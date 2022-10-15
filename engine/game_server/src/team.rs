// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game_service::GameArenaService;
use crate::ordered_set::OrderedSet;
use crate::player::{PlayerData, PlayerRepo};
use crate::unwrap_or_return;
use crate::util::diff_small_n;
use atomic_refcell::AtomicRefMut;
use core_protocol::dto::TeamDto;
use core_protocol::id::{PlayerId, TeamId};
use core_protocol::name::TeamName;
use core_protocol::rpc::{TeamRequest, TeamUpdate};
use server_util::generate_id::generate_id;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::sync::Arc;

/// Data stored for team.
pub struct TeamData<G: GameArenaService> {
    pub name: TeamName,
    /// In order of join, first is captain.
    pub members: OrderedSet<PlayerId>,
    /// In order of request. They are never reordered.
    joiners: OrderedSet<PlayerId>,
    _spooky: PhantomData<G>,
}

impl<G: GameArenaService> TeamData<G> {
    /// Creates a new team.
    pub fn new(name: TeamName, captain: PlayerId) -> Self {
        Self {
            name,
            members: OrderedSet::new_with_one(captain),
            joiners: OrderedSet::new(),
            _spooky: PhantomData,
        }
    }

    /// Returns whether player_id a member of the team.
    pub fn is_member(&self, player_id: PlayerId) -> bool {
        debug_assert!(self.members.len() > 0, "team shouldn't be empty");
        self.members.contains(player_id)
    }

    /// Returns whether player_id a the team captain.
    pub fn is_captain(&self, player_id: PlayerId) -> bool {
        debug_assert!(
            self.members.peek_front().is_some(),
            "team shouldn't be empty"
        );
        self.members.peek_front() == Some(player_id)
    }

    /// Returns if the team has the maximum possible amount of members.
    pub fn is_full(&self, players_online: usize) -> bool {
        self.members.len() >= G::team_members_max(players_online)
    }

    /// Returns if the team has the maximum possible amount of joiners a.k.a. requests.
    pub fn is_closed(&self) -> bool {
        self.joiners.len() >= G::TEAM_JOINERS_MAX
    }

    /// Returns whether player is now captain (regardless of whether a swap was necessary),
    /// i.e. only returns false if player wasn't a member.
    ///
    /// Note: Automatically updates whether members changed.
    pub fn assign_captain(&mut self, player_id: PlayerId) -> bool {
        if self.members.swap_to_front(player_id) {
            true
        } else {
            // Player not in team.
            false
        }
    }
}

/// Data related to team, stored in player.
#[derive(Debug, Default)]
pub struct PlayerTeamData {
    status: PlayerTeamStatus,
    /// The last [`TeamId`] the game service was informed of via [`GameArenaService::player_changed_team`].
    pub(crate) previous_team_id: Option<TeamId>,
}

/// Data related to team, stored in client.
#[derive(Debug, Default)]
pub struct ClientTeamData {
    /// For diffing.
    previous_members: OrderedSet<PlayerId>,
    /// For diffing.
    previous_joiners: OrderedSet<PlayerId>,
    /// For diffing.
    previous_joins: VecDeque<TeamId>,
}

impl Drop for PlayerTeamData {
    fn drop(&mut self) {
        debug_assert!(
            std::thread::panicking() || matches!(self.status, PlayerTeamStatus::Solo { .. }),
            "player must be solo when forgotten, to maintain team structure"
        );
    }
}

#[derive(Debug)]
pub enum PlayerTeamStatus {
    Teamed {
        /// Team player is currently a member of.
        team_id: TeamId,
    },
    Solo {
        /// Teams player is requesting to join.
        joins: VecDeque<TeamId>,
    },
}

impl Default for PlayerTeamStatus {
    fn default() -> Self {
        Self::solo()
    }
}

impl PlayerTeamStatus {
    fn solo() -> Self {
        Self::Solo {
            joins: VecDeque::default(),
        }
    }

    fn teamed(team_id: TeamId) -> Self {
        Self::Teamed { team_id }
    }
}

impl PlayerTeamData {
    /// Gets [`TeamId`] if teamed, otherwise [`None`].
    pub fn team_id(&self) -> Option<TeamId> {
        match &self.status {
            PlayerTeamStatus::Teamed { team_id } => Some(*team_id),
            PlayerTeamStatus::Solo { .. } => None,
        }
    }
}

impl ClientTeamData {
    /// Call when it is reasonable to assume the client forgot past information.
    pub fn forget_state(&mut self) {
        *self = Self::default();
    }
}

/// Part of [`Context`] relating to teams.
pub struct TeamRepo<G: GameArenaService> {
    teams: HashMap<TeamId, TeamData<G>>,
    previous: Arc<[TeamDto]>,
    _spooky: PhantomData<G>,
}

impl<G: GameArenaService> TeamRepo<G> {
    pub fn new() -> Self {
        Self {
            teams: HashMap::new(),
            previous: Vec::new().into(),
            _spooky: PhantomData,
        }
    }

    /// Gets immutable reference to team.
    pub fn get(&self, team_id: TeamId) -> Option<&TeamData<G>> {
        self.teams.get(&team_id)
    }

    fn accept_or_reject_player(
        &mut self,
        req_player_id: PlayerId,
        joiner_player_id: PlayerId,
        accept: bool,
        players: &mut PlayerRepo<G>,
    ) -> Result<TeamUpdate, &'static str> {
        if joiner_player_id == req_player_id {
            return Err("cannot accept/reject self");
        }

        let req_player = players
            .borrow_player(req_player_id)
            .ok_or("requesting player nonexistent")?;

        let team_id = req_player.team_id().ok_or("not in team")?;

        let team = self.teams.get_mut(&team_id).ok_or_else(|| {
            debug_assert!(false, "team id should have been cleared");
            "nonexistent team"
        })?;
        if !team.is_captain(req_player_id) {
            return Err("not captain");
        }
        if accept && team.is_full(players.real_players_live) {
            return Err("team full");
        }
        if !team.joiners.remove(joiner_player_id) {
            return Err("player wasn't requesting to join");
        }

        let mut joiner_player = players
            .borrow_player_mut(joiner_player_id)
            .ok_or("nonexistent player")?;

        if joiner_player.team_id().is_some() {
            debug_assert!(false, "should have been removed from joiners");
            return Err("cannot accept/reject player already on team");
        }

        Ok(if accept {
            // At this point, all checks have passed, and we are committed to accepting the player\
            // into the team.
            team.members.insert_back(joiner_player_id);

            debug_assert!(
                team.members.len() <= G::team_members_max(players.real_players_live),
                "team overfull"
            );

            self.assign_team_and_cancel_joins(joiner_player, team_id);

            TeamUpdate::Accepted(joiner_player_id)
        } else {
            // We already removed the player from joiners, just remove join and report success.
            if let PlayerTeamStatus::Solo { joins } = &mut joiner_player.team.status {
                if !remove_join(team_id, joins) {
                    debug_assert!(false, "no join, but successfully rejected");
                }
            } else {
                unreachable!("already returned error if joiner player was teamed")
            }
            TeamUpdate::Rejected(joiner_player_id)
        })
    }

    /// Changes the player's team status from solo to teamed, with the specified team id. Cancels
    /// all pending joins.
    ///
    /// Note: It is ok to clear the "joiner" from the team being joined prior to calling this.
    ///
    /// # Panics
    ///
    /// If player's team status isn't solo.
    fn assign_team_and_cancel_joins(
        &mut self,
        mut formerly_solo_player: AtomicRefMut<PlayerData<G>>,
        joining_team_id: TeamId,
    ) {
        let old_status = std::mem::replace(
            &mut formerly_solo_player.team.status,
            PlayerTeamStatus::teamed(joining_team_id),
        );

        if let PlayerTeamStatus::Solo { joins } = old_status {
            // Revoke all joins.
            for revoke_join_team_id in joins {
                if let Some(team) = self.teams.get_mut(&revoke_join_team_id) {
                    let was_present = team.joiners.remove(formerly_solo_player.player_id);
                    debug_assert!(
                        was_present || revoke_join_team_id == joining_team_id,
                        "joiner player wasn't in joiners"
                    );
                } else {
                    debug_assert!(false, "joining non-existent team");
                }
            }
        } else {
            unreachable!("status was supposed to be solo");
        }
    }

    fn promote_player(
        &mut self,
        req_player_id: PlayerId,
        assign_player_id: PlayerId,
        players: &mut PlayerRepo<G>,
    ) -> Result<TeamUpdate, &'static str> {
        if assign_player_id == req_player_id {
            return Err("cannot assign self");
        }

        let req_player = players
            .borrow_player(req_player_id)
            .ok_or("requesting player nonexistent")?;

        let team_id = req_player.team_id().ok_or("not in team")?;
        let team = self.teams.get_mut(&team_id).ok_or_else(|| {
            debug_assert!(false, "team id should have been cleared");
            "nonexistent team"
        })?;
        if !team.is_captain(req_player_id) {
            return Err("not captain");
        }
        // This updates members_changed automatically.
        if team.assign_captain(assign_player_id) {
            Ok(TeamUpdate::Promoted(assign_player_id))
        } else {
            Err("can only assign team members to be captain")
        }
    }

    fn create_team(
        &mut self,
        req_player_id: PlayerId,
        team_name: TeamName,
        players: &mut PlayerRepo<G>,
    ) -> Result<TeamUpdate, &'static str> {
        if G::team_members_max(players.real_players_live) == 0 {
            return Err("teams are currently disabled");
        }

        let req_player = players
            .borrow_player_mut(req_player_id)
            .ok_or("requesting player nonexistent")?;

        if !req_player.is_alive() {
            return Err("must be alive to create team");
        }

        if req_player.team_id().is_some() {
            return Err("already in team");
        }

        let censored_team_name = TeamName::new_sanitized(team_name.as_str());

        if censored_team_name.is_empty() {
            return Err("cannot use empty team name");
        }
        if self.teams.values().any(|t| t.name == censored_team_name) {
            return Err("team name in use");
        }

        let team_data = TeamData::new(censored_team_name, req_player_id);

        let team_id = loop {
            let team_id = TeamId(generate_id());
            if let Entry::Vacant(e) = self.teams.entry(team_id) {
                e.insert(team_data);
                break team_id;
            }
        };

        self.assign_team_and_cancel_joins(req_player, team_id);

        Ok(TeamUpdate::Created(team_id, censored_team_name))
    }

    fn kick_player(
        &mut self,
        req_player_id: PlayerId,
        kick_player_id: PlayerId,
        players: &mut PlayerRepo<G>,
    ) -> Result<TeamUpdate, &'static str> {
        if kick_player_id == req_player_id {
            return Err("cannot kick self");
        }

        let req_player = players
            .borrow_player(req_player_id)
            .ok_or("requesting player nonexistent")?;

        let team_id = req_player.team_id().ok_or("not in team")?;
        let team = self.teams.get_mut(&team_id).ok_or_else(|| {
            debug_assert!(false, "team id should have been cleared");
            "nonexistent team"
        })?;
        if !team.is_captain(req_player_id) {
            return Err("not captain");
        }
        let mut kick_player = players
            .borrow_player_mut(kick_player_id)
            .ok_or("nonexistent player")?;

        if team.members.remove(kick_player_id) {
            debug_assert_eq!(kick_player.team_id(), Some(team_id));
            debug_assert!(!team.members.is_empty(), "kick reduced team to no members");
            kick_player.team.status = PlayerTeamStatus::solo();

            Ok(TeamUpdate::Kicked(kick_player_id))
        } else {
            Err("cannot kick player that isn't in team")
        }
    }

    /// Call when a player requests to quit their team.
    pub(crate) fn quit_team(
        &mut self,
        req_player_id: PlayerId,
        players: &PlayerRepo<G>,
    ) -> Result<TeamUpdate, &'static str> {
        let mut req_player = players
            .borrow_player_mut(req_player_id)
            .ok_or("requesting player nonexistent")?;

        let team_id = req_player.team_id().ok_or("not in team")?;
        req_player.team.status = PlayerTeamStatus::solo();

        // We may need to borrow other players later on, and we are done with this one.
        drop(req_player);

        let team = self.teams.get_mut(&team_id).ok_or_else(|| {
            debug_assert!(false, "team id should have been cleared");
            "nonexistent team"
        })?;

        if team.members.remove(req_player_id) {
            if team.members.is_empty() {
                // Last one to leave, delete the team, starting with its joiners.
                for joiner_player_id in team.joiners.iter() {
                    if let Some(mut joiner_player) = players.borrow_player_mut(joiner_player_id) {
                        match &mut joiner_player.team.status {
                            PlayerTeamStatus::Teamed { .. } => {
                                debug_assert!(false, "joiner of deleted team isn't solo");
                            }
                            PlayerTeamStatus::Solo { joins } => {
                                if !remove_join(team_id, joins) {
                                    debug_assert!(
                                        false,
                                        "joiner of deleted team wasn't present in joiners"
                                    );
                                }
                            }
                        }
                    } else {
                        debug_assert!(false, "joiner of deleted team doesn't exist");
                    }
                }

                // Now, actually delete the team.
                let deleted = self.teams.remove(&team_id);
                debug_assert!(deleted.is_some());
            }
        } else {
            debug_assert!(false, "quit team, but wasn't a member");
        }

        Ok(TeamUpdate::Left)
    }

    pub(crate) fn request_join(
        &mut self,
        req_player_id: PlayerId,
        join_team_id: TeamId,
        players: &mut PlayerRepo<G>,
    ) -> Result<TeamUpdate, &'static str> {
        let mut req_player = players
            .borrow_player_mut(req_player_id)
            .ok_or("requesting player nonexistent")?;

        if !req_player.is_alive() {
            return Err("must be alive to join team");
        }

        let join_team = self
            .teams
            .get_mut(&join_team_id)
            .ok_or("nonexistent team")?;

        if join_team.is_closed() {
            return Err("team is closed to requests");
        }

        let joins = match &mut req_player.team.status {
            PlayerTeamStatus::Teamed { .. } => return Err("cannot request when already on team"),
            PlayerTeamStatus::Solo { joins } => joins,
        };

        let no_longer_joining = if joins.contains(&join_team_id) {
            return Err("already requested to join this team");
        } else if joins.len() >= G::TEAM_JOINS_MAX {
            // Make room in joins.
            joins.pop_front()
        } else {
            None
        };

        joins.push_back(join_team_id);

        if !join_team.joiners.insert_back(req_player_id) {
            debug_assert!(
                false,
                "team id wasn't in joins, but player id was in joiners"
            );
        }

        if let Some(no_longer_joining_team) = no_longer_joining.and_then(|id| {
            let team = self.teams.get_mut(&id);
            debug_assert!(team.is_some(), "team gone in no longer joining");
            team
        }) {
            let was_present = no_longer_joining_team.joiners.remove(req_player_id);
            debug_assert!(
                was_present,
                "player isn't present in joiners of team they are no longer joining"
            );
        }
        Ok(TeamUpdate::Joining(join_team_id))
    }

    pub(crate) fn handle_team_request(
        &mut self,
        req_player_id: PlayerId,
        request: TeamRequest,
        players: &mut PlayerRepo<G>,
    ) -> Result<TeamUpdate, &'static str> {
        match request {
            TeamRequest::Accept(player_id) => {
                self.accept_or_reject_player(req_player_id, player_id, true, players)
            }
            TeamRequest::Promote(player_id) => {
                self.promote_player(req_player_id, player_id, players)
            }
            TeamRequest::Create(name) => self.create_team(req_player_id, name, players),
            TeamRequest::Kick(player_id) => self.kick_player(req_player_id, player_id, players),
            TeamRequest::Leave => self.quit_team(req_player_id, players),
            TeamRequest::Reject(player_id) => {
                self.accept_or_reject_player(req_player_id, player_id, false, players)
            }
            TeamRequest::Join(team_id) => self.request_join(req_player_id, team_id, players),
        }
    }

    /// Gets initializer for new client.
    pub(crate) fn initializer(&self) -> Option<TeamUpdate> {
        (!self.previous.is_empty()).then(|| TeamUpdate::AddedOrUpdated(Arc::clone(&self.previous)))
    }

    /// Computes current set of team dtos.
    fn compute_team_dtos(&self, players: &PlayerRepo<G>) -> Vec<TeamDto> {
        self.teams
            .iter()
            .map(|(&team_id, team_data)| TeamDto {
                team_id,
                name: team_data.name,
                full: team_data.is_full(players.real_players_live),
                closed: team_data.is_closed(),
            })
            .collect()
    }

    /// Computes a diff, and updates cached dtos.
    pub(crate) fn delta(
        &mut self,
        players: &PlayerRepo<G>,
    ) -> Option<(Arc<[TeamDto]>, Arc<[TeamId]>)> {
        let current_players = self.compute_team_dtos(players);

        if let Some((added, removed)) =
            diff_small_n(&self.previous, &current_players, |dto| dto.team_id)
        {
            self.previous = current_players.into();
            Some((added.into(), removed.into()))
        } else {
            None
        }
    }

    /// Return delta in members, joiners, and joins for a given player.
    /// Only returns [`None`] at the outer level if the player doesn't exist or isn't a real player.
    pub(crate) fn player_delta(
        &mut self,
        player_id: PlayerId,
        players: &PlayerRepo<G>,
    ) -> Option<(
        Option<OrderedSet<PlayerId>>,
        Option<OrderedSet<PlayerId>>,
        Option<VecDeque<TeamId>>,
    )> {
        let mut player = players.borrow_player_mut(player_id)?;
        let player = &mut *player;

        // Avoid allocations by handing out references to these.
        static EMPTY_PLAYERS: OrderedSet<PlayerId> = OrderedSet::new();
        lazy_static::lazy_static! {
            static ref EMPTY_TEAMS: VecDeque<TeamId> = VecDeque::new();
        }

        // Help out the borrow checker a bit.
        let client = player.client.as_deref_mut()?;
        let team = &mut client.team;
        let previous_members = &mut team.previous_members;
        let previous_joiners = &mut team.previous_joiners;
        let previous_joins = &mut team.previous_joins;

        let (members, joiners, joins) = match &player.team.status {
            PlayerTeamStatus::Teamed { team_id } => {
                if let Some(team) = self.teams.get_mut(team_id) {
                    let joiners = if team.is_captain(player_id) {
                        &team.joiners
                    } else {
                        &EMPTY_PLAYERS
                    };

                    // In a team, not joining any other team.
                    (&team.members, joiners, &*EMPTY_TEAMS)
                } else {
                    debug_assert!(false, "player's team doesn't exist");
                    (&EMPTY_PLAYERS, &EMPTY_PLAYERS, &*EMPTY_TEAMS)
                }
            }
            PlayerTeamStatus::Solo { joins } => {
                // Not in a team, don't have members or joiners.
                (&EMPTY_PLAYERS, &EMPTY_PLAYERS, joins)
            }
        };

        Some((
            (members != previous_members).then(|| {
                *previous_members = members.clone();
                members.clone()
            }),
            (joiners != previous_joiners).then(|| {
                *previous_joiners = joiners.clone();
                joiners.clone()
            }),
            (joins != previous_joins).then(|| {
                *previous_joins = joins.clone();
                joins.clone()
            }),
        ))
    }

    /// Call when a player abandoned the game. This has the effect of removing player from their
    /// team, if any.
    ///
    /// This must be called at least once before the player is forgotten, but may be called earlier
    /// than that.
    pub(crate) fn cleanup_player(&mut self, player_id: PlayerId, players: &PlayerRepo<G>) {
        let _ = self.quit_team(player_id, players);
        for team in self.teams.values_mut() {
            team.joiners.remove(player_id);
        }
        let mut player = unwrap_or_return!(players.borrow_player_mut(player_id));
        if let PlayerTeamStatus::Solo { joins } = &mut player.team.status {
            joins.clear();
        } else {
            debug_assert!(false, "player still teamed");
        }
    }
}

/// Returns whether existed and was removed.
fn remove_join(join: TeamId, joins: &mut VecDeque<TeamId>) -> bool {
    if let Some(idx) = joins.iter().position(|&id| id == join) {
        joins.remove(idx);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod test {
    use crate::game_service::{GameArenaService, MockGame};
    use crate::player::{PlayerData, PlayerRepo, PlayerTuple};
    use crate::team::TeamRepo;
    use core_protocol::id::{PlayerId, TeamId};
    use core_protocol::name::TeamName;
    use core_protocol::rpc::{TeamRequest, TeamUpdate};
    use rand::{prelude::IteratorRandom, thread_rng, Rng};
    use server_util::generate_id::generate_id;
    use std::sync::Arc;

    #[test]
    fn team_repo() {
        let mut players = PlayerRepo::<MockGame>::new();
        let mut teams = TeamRepo::<MockGame>::new();

        let nonexistent_player_id = PlayerId::nth_bot(0).unwrap();

        let mut existing_players = Vec::new();

        for i in 1..50 {
            let existing_player_id = PlayerId::nth_bot(i).unwrap();
            let existing_player = Arc::new(PlayerTuple::<MockGame>::new(PlayerData::new(
                existing_player_id,
                None,
            )));
            existing_player.borrow_player_mut().was_alive = true;
            players.insert(existing_player_id, Arc::clone(&existing_player));
            existing_players.push((existing_player_id, existing_player));
        }

        assert_eq!(teams.teams.len(), 0);

        assert!(teams
            .handle_team_request(
                nonexistent_player_id,
                TeamRequest::Create(TeamName::new_sanitized("test")),
                &mut players
            )
            .is_err());

        assert_eq!(teams.teams.len(), 0);

        let team1_id = match teams.handle_team_request(
            existing_players[0].0,
            TeamRequest::Create(TeamName::new_sanitized("test1")),
            &mut players,
        ) {
            Ok(TeamUpdate::Created(team1_id, _)) => team1_id,
            Err(e) => unreachable!("{:?}", e),
            _ => unreachable!(),
        };

        assert_eq!(teams.teams.len(), 1);

        let res = teams.handle_team_request(
            existing_players[0].0,
            TeamRequest::Create(TeamName::new_sanitized("test2")),
            &mut players,
        );
        assert!(res.is_err(), "{:?}", res);

        let res = teams.handle_team_request(
            existing_players[1].0,
            TeamRequest::Create(TeamName::new_sanitized("test1")),
            &mut players,
        );
        assert!(res.is_err(), "{:?}", res);

        let _team2_id = if let Ok(TeamUpdate::Created(team2_id, _)) = teams.handle_team_request(
            existing_players[1].0,
            TeamRequest::Create(TeamName::new_sanitized("test2")),
            &mut players,
        ) {
            team2_id
        } else {
            unreachable!();
        };

        assert_eq!(teams.teams.len(), 2);

        for i in 20..20 + MockGame::TEAM_JOINERS_MAX {
            let res = teams.handle_team_request(
                existing_players[i].0,
                TeamRequest::Join(team1_id),
                &mut players,
            );
            assert!(matches!(res, Ok(TeamUpdate::Joining(_))), "{:?}", res);
        }

        for i in 5..10 {
            let res = teams.handle_team_request(
                existing_players[i].0,
                TeamRequest::Join(team1_id),
                &mut players,
            );
            assert!(res.is_err(), "{:?}", res);
        }

        // Remove first two players.
        let res =
            teams.handle_team_request(existing_players[1].0, TeamRequest::Leave, &mut players);
        assert!(matches!(res, Ok(TeamUpdate::Left)), "{:?}", res);

        assert_eq!(teams.teams.len(), 1);

        let res =
            teams.handle_team_request(existing_players[0].0, TeamRequest::Leave, &mut players);
        assert!(matches!(res, Ok(TeamUpdate::Left)), "{:?}", res);

        assert_eq!(teams.teams.len(), 0);
    }

    #[test]
    fn fuzz() {
        let mut players = PlayerRepo::<MockGame>::new();
        let mut teams = TeamRepo::<MockGame>::new();

        let mut existing_players = Vec::new();

        for i in 5..75 {
            let existing_player_id = PlayerId::nth_bot(i).unwrap();
            let mut player_data = PlayerData::new(existing_player_id, None);
            player_data.was_alive = thread_rng().gen_bool(0.8);
            let existing_player = Arc::new(PlayerTuple::<MockGame>::new(player_data));
            players.insert(existing_player_id, Arc::clone(&existing_player));
            existing_players.push((existing_player_id, existing_player));
        }

        let team_names = vec![
            TeamName::new_sanitized("one"),
            TeamName::new_sanitized("two"),
            TeamName::new_sanitized("three"),
        ];

        for _ in 0..50000 {
            let rand_player_id_1 = PlayerId::nth_bot(thread_rng().gen_range(0..50)).unwrap();
            let rand_player_id_2 = PlayerId::nth_bot(thread_rng().gen_range(25..80)).unwrap();

            let req = match thread_rng().gen_range(0..8) {
                0 => TeamRequest::Leave,
                1 => TeamRequest::Create(*team_names.iter().choose(&mut thread_rng()).unwrap()),
                2 => {
                    let mut team_ids: Vec<TeamId> = teams.teams.keys().cloned().collect();
                    team_ids.push(TeamId(generate_id()));
                    TeamRequest::Join(*team_ids.iter().choose(&mut thread_rng()).unwrap())
                }
                3 => TeamRequest::Accept(rand_player_id_1),
                4 => TeamRequest::Reject(rand_player_id_1),
                5 => TeamRequest::Kick(rand_player_id_1),
                6 => TeamRequest::Promote(rand_player_id_1),
                _ => {
                    teams.cleanup_player(rand_player_id_1, &mut players);
                    continue;
                }
            };

            assert!(teams.teams.len() <= team_names.len());

            let _ = teams.handle_team_request(rand_player_id_2, req, &mut players);
        }

        for player_id in players.iter_player_ids().collect::<Vec<_>>() {
            let _ = teams.handle_team_request(player_id, TeamRequest::Leave, &mut players);
        }
    }
}
