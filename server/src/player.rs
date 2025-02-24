// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entities::*;
use crate::server::PlayerExtension;
use crate::team::{PlayerTeamData, TeamRepo};
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use common::death_reason::DeathReason;
use common::protocol::Hint;
use kodiak_server::glam::Vec2;
use kodiak_server::{ArenaService, PlayerAlias, PlayerId, RankNumber, TeamId};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Instant;

/// A player's view into the world.
#[allow(dead_code)]
pub struct Camera {
    pub center: Vec2,
    pub radius: f32,
}

/// Set based on player inputs.
/// Cleared each physics tick.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Flags {
    /// Player just left the game so all of it's entities should be removed.
    pub left_game: bool,
    /// Player just left a team that has other players so all mines should be removed.
    pub left_populated_team: bool,
    /// Player just upgraded and all limited entities should be removed.
    pub upgraded: bool,
}

/// Status is an enumeration of mutually exclusive player states.
#[derive(Debug)]
pub enum Status {
    /// Player has a boat.
    Alive {
        /// Index of player's boat in world.entities.
        entity_index: EntityIndex,
        /// Where the player is aiming. Used by turrets and aircraft.
        aim_target: Option<Vec2>,
    },
    /// Player had a boat.
    Dead {
        /// Why they died.
        reason: DeathReason,
        /// Where they died.
        position: Vec2,
        /// When they died (for spawn exclusion and bandwidth saving).
        time: Instant,
        /// How far they could see when they died.
        visual_range: f32,
    },
    /// Player never had a boat.
    Spawning,
}

impl Status {
    pub fn new_alive(entity_index: EntityIndex) -> Self {
        Self::Alive {
            entity_index,
            aim_target: None,
        }
    }

    /// is_alive returns whether the status matches Status::Alive.
    pub fn is_alive(&self) -> bool {
        matches!(self, Status::Alive { .. })
    }

    /// is_alive returns whether the status matches Status::Respawning.
    pub fn is_dead(&self) -> bool {
        matches!(self, Status::Dead { .. })
    }

    /// Returns entity index if alive, otherwise none.
    /// Doesn't consider `Flags::left_game`.
    #[cfg(test)]
    pub fn get_entity_index(&self) -> Option<EntityIndex> {
        match self {
            Self::Alive { entity_index, .. } => Some(*entity_index),
            _ => None,
        }
    }

    /// set_entity_index sets the entity index of an Alive status or panics if the status is not alive.
    pub fn set_entity_index(&mut self, new_index: EntityIndex) {
        if let Self::Alive { entity_index, .. } = self {
            *entity_index = new_index;
        } else {
            panic!(
                "set_entity_index() called on a non-alive status of {:?}",
                self
            );
        }
    }
}

/// Player is the owner of a boat, either a real person or a bot.
#[derive(Debug)]
pub struct TempPlayer {
    pub player_id: PlayerId,
    pub alias: PlayerAlias,
    pub rank: Option<RankNumber>,
    pub score: u32,
    pub team: PlayerTeamData,
    /// Flags set each tick based on inputs.
    /// Only cleared if player has a boat.
    /// Cleared once when the boat is spawn and once in each physics tick.
    pub flags: Flags,
    /// Hints from client.
    pub hint: Hint,
    /// Current status e.g. Alive, Dead, or Spawning.
    pub status: Status,
}

impl TempPlayer {
    /// new allocates a player with Status::Spawning.
    pub fn new(player_id: PlayerId, rank: Option<RankNumber>) -> Self {
        Self {
            player_id,
            alias: PlayerAlias::default(),
            rank,
            score: 0,
            team: Default::default(),
            flags: Flags::default(),
            hint: Hint::default(),
            status: Status::Spawning,
        }
    }

    pub fn is_bot(&self) -> bool {
        self.player_id.is_bot()
    }

    pub fn team_id(&self) -> Option<TeamId> {
        self.team.team_id()
    }

    pub fn is_alive(&self) -> bool {
        self.status.is_alive()
    }
}

pub struct PlayerTuple {
    pub player: AtomicRefCell<TempPlayer>,
    pub extension: PlayerExtension,
}

impl PlayerTuple {
    pub fn new(player: TempPlayer) -> Self {
        PlayerTuple {
            player: AtomicRefCell::new(player),
            extension: Default::default(),
        }
    }
}

impl PlayerTuple {
    /// Borrows the player.
    pub fn borrow_player(&self) -> AtomicRef<TempPlayer> {
        self.player.borrow()
    }

    /// Mutably borrows the player.
    pub fn borrow_player_mut(&self) -> AtomicRefMut<TempPlayer> {
        self.player.borrow_mut()
    }

    /// # Safety
    /// Borrows the player without checking for outstanding mutable borrows, the existence of which
    /// would cause undefined behavior.
    #[allow(unused)]
    pub unsafe fn borrow_player_unchecked(&self) -> &TempPlayer {
        #[cfg(debug_assertions)]
        drop(self.borrow_player());
        &*self.player.as_ptr()
    }

    /// # Safety
    /// Mutably borrows the player without checking for outstanding borrows, the existence
    /// of which would cause undefined behavior.
    #[allow(clippy::mut_from_ref)]
    #[allow(unused)]
    pub unsafe fn borrow_player_mut_unchecked(&self) -> &mut TempPlayer {
        #[cfg(debug_assertions)]
        drop(self.borrow_player_mut());
        &mut *self.player.as_ptr()
    }
}

impl PartialEq for PlayerTuple {
    fn eq(&self, other: &Self) -> bool {
        self.player.as_ptr() == other.player.as_ptr()
    }
}

impl Debug for PlayerTuple {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.borrow_player())
    }
}

#[derive(Default)]
pub struct PlayerTupleRepo {
    /// Ground-truth player data. Care must be exercised to avoid mutably borrowing the same player
    /// twice, which will induce a runtime error.
    pub(crate) players: HashMap<PlayerId, Arc<PlayerTuple>>,
    /// Copied from [`PlayerRepo`] (TODO)
    pub(crate) real_players_live: u16,
}

impl PlayerTupleRepo {
    /// Returns total number of players (including bots).
    #[allow(clippy::len_without_is_empty)]
    #[allow(unused)]
    pub fn len(&self) -> usize {
        self.players.len()
    }

    /// Tests if the player exists (in cache).
    pub fn contains(&self, player_id: PlayerId) -> bool {
        self.players.contains_key(&player_id)
    }

    /// Gets the player tuple of a given player.
    pub fn get(&self, player_id: PlayerId) -> Option<&Arc<PlayerTuple>> {
        self.players.get(&player_id)
    }

    /// Inserts a player (it is not mandatory to insert this way).
    pub(crate) fn insert(&mut self, player_id: PlayerId, player: Arc<PlayerTuple>) {
        #[cfg(debug_assertions)]
        {
            if let Some(existing) = self.players.get(&player_id) {
                assert_eq!(existing.borrow_player().player_id, player_id);
            }
        }
        self.players.insert(player_id, player);
    }

    /// Removes a player, performing mandatory cleanup steps.
    pub(crate) fn forget<G: ArenaService>(&mut self, player_id: PlayerId, teams: &mut TeamRepo<G>) {
        teams.cleanup_player(player_id, self);
        self.players.remove(&player_id);
    }

    /// Cannot coincide with mutable references to players.
    pub fn borrow_player(&self, player_id: PlayerId) -> Option<AtomicRef<TempPlayer>> {
        self.get(player_id).map(|pt| pt.borrow_player())
    }

    /// Cannot coincide with other references to players.
    pub fn borrow_player_mut(&self, player_id: PlayerId) -> Option<AtomicRefMut<TempPlayer>> {
        self.get(player_id).map(|pt| pt.borrow_player_mut())
    }

    /// Iterates every player tuple (real and bot).
    #[allow(unused)]
    pub fn iter(&self) -> impl Iterator<Item = &Arc<PlayerTuple>> {
        self.players.values()
    }

    /// Iterates every player id (real and bot).
    #[allow(unused)]
    pub fn iter_player_ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.players.keys().cloned()
    }

    /// Iterates every player tuple, immutably borrowing it automatically.
    /// Cannot coincide with mutable references to players.
    #[allow(unused)]
    pub fn iter_borrow(&self) -> impl Iterator<Item = AtomicRef<TempPlayer>> {
        self.players.values().map(|pt| pt.borrow_player())
    }

    /// Iterates every player tuple, mutably borrowing it automatically.
    /// Cannot coincide with other references to players.
    #[allow(unused)]
    pub fn iter_borrow_mut(&mut self) -> impl Iterator<Item = AtomicRefMut<TempPlayer>> {
        self.players.values().map(|pt| pt.borrow_player_mut())
    }
}
