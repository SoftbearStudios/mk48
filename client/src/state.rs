// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::animation::Animation;
use crate::interpolated_contact::InterpolatedContact;
use client_util::apply::Apply;
use common::contact::Contact;
use common::death_reason::DeathReason;
use common::entity::EntityId;
use common::protocol::Update;
use common::terrain::Terrain;
use std::collections::HashMap;

/// State associated with game server connection. Reset when connection is reset.
#[derive(Default)]
pub struct Mk48State {
    pub score: u32,
    pub entity_id: Option<EntityId>,
    pub contacts: HashMap<EntityId, InterpolatedContact>,
    pub animations: Vec<Animation>,
    pub terrain: Terrain,
    pub world_radius: f32,
    pub death_reason: Option<DeathReason>,
}

impl Mk48State {
    /// Returns the "view" of the player's boat's contact, if the player has a boat.
    pub(crate) fn player_contact(&self) -> Option<&Contact> {
        self.entity_id
            .map(|id| &self.contacts.get(&id).unwrap().view)
    }
}

impl Apply<Update> for Mk48State {
    fn apply(&mut self, update: Update) {
        self.death_reason = update.death_reason;
        self.terrain.apply_update(&update.terrain);
        self.world_radius = update.world_radius;
        self.score = update.score;
    }
}
