// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entities::*;
use common::death_reason::DeathReason;
use common::protocol::Hint;
use glam::Vec2;
use std::fmt::Debug;
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
pub struct Player {
    /// Flags set each tick based on inputs.
    /// Only cleared if player has a boat.
    /// Cleared once when the boat is spawn and once in each physics tick.
    pub flags: Flags,
    /// Hints from client.
    pub hint: Hint,
    /// Current status e.g. Alive, Dead, or Spawning.
    pub status: Status,
}

impl Default for Player {
    /// new allocates a player with Status::Spawning.
    fn default() -> Self {
        Self {
            flags: Flags::default(),
            hint: Hint::default(),
            status: Status::Spawning,
        }
    }
}
