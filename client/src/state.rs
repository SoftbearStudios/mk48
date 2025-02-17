// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::animation::Animation;
use crate::interpolated_contact::InterpolatedContact;
use common::contact::Contact;
use common::death_reason::DeathReason;
use common::entity::EntityId;
use common::protocol::{TeamDto, TeamUpdate, Update};
use common::terrain::Terrain;
use kodiak_client::{owned_into_box, owned_into_iter, Apply, PlayerId, TeamId};
use std::collections::HashMap;

/// State associated with game server connection. Reset when connection is reset.
pub struct Mk48State {
    pub animations: Vec<Animation>,
    pub contacts: HashMap<EntityId, InterpolatedContact>,
    pub death_reason: Option<DeathReason>,
    pub entity_id: Option<EntityId>,
    pub score: u32,
    pub terrain: Terrain,
    pub world_radius: f32,
    terrain_reset: bool,
    pub teams: HashMap<TeamId, TeamDto>,
    /// Ordered, i.e. first is captain.
    pub members: Box<[PlayerId]>,
    pub joiners: Box<[PlayerId]>,
    pub joins: Box<[TeamId]>,
}

impl Default for Mk48State {
    fn default() -> Self {
        Self {
            animations: Vec::new(),
            contacts: HashMap::new(),
            death_reason: None,
            entity_id: None,
            score: 0,
            terrain: Terrain::default(),
            // Keep border off splash screen by assuming radius.
            world_radius: 10000.0,
            terrain_reset: false,
            teams: Default::default(),
            members: Default::default(),
            joiners: Default::default(),
            joins: Default::default(),
        }
    }
}

impl Mk48State {
    /// Returns the "view" of the player's boat's contact, if the player has a boat.
    pub(crate) fn player_contact(&self) -> Option<&Contact> {
        self.entity_id
            .map(|id| &self.contacts.get(&id).unwrap().view)
    }

    pub(crate) fn player_interpolated_contact(&self) -> Option<&InterpolatedContact> {
        self.entity_id.map(|id| self.contacts.get(&id).unwrap())
    }

    // Reset terrain cache when switching servers and state resets.
    // TODO find a better way to do this.
    pub fn take_terrain_reset(&mut self) -> bool {
        if self.terrain_reset {
            self.terrain_reset = false;
            true
        } else {
            false
        }
    }
}

impl Apply<Update> for Mk48State {
    fn apply(&mut self, update: Update) {
        self.death_reason = update.death_reason;

        // Didn't consume previous update (tabbed out) and now terrain updated state is invalid.
        self.terrain_reset = !self.terrain.updated.is_empty();
        self.terrain.apply_update(&update.terrain);

        self.world_radius = update.world_radius;
        self.score = update.score;

        for update in update.team {
            match update {
                TeamUpdate::Members(members) => {
                    self.members = owned_into_box(members);
                }
                TeamUpdate::Joiners(joiners) => {
                    self.joiners = joiners;
                }
                TeamUpdate::Joins(joins) => {
                    self.joins = joins;
                }
                TeamUpdate::AddedOrUpdated(added_or_updated) => {
                    for team in owned_into_iter(added_or_updated) {
                        self.teams.insert(team.team_id, team);
                    }
                }
                TeamUpdate::Removed(removed) => {
                    for team_id in removed.iter() {
                        self.teams.remove(team_id);
                    }
                }
                _ => {}
            }
        }
    }

    fn reset(&mut self) {
        *self = Self {
            terrain_reset: true,
            ..Self::default()
        };
    }
}
