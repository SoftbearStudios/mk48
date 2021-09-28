// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::contact_ref::ContactRef;
use crate::player::Player;
use crate::player::Status;
use crate::world::World;
use atomic_refcell::AtomicRef;
use common::complete::CompleteTrait;
use common::contact::ContactTrait;
use common::death_reason::DeathReason;
use common::entity::{EntityId, EntityKind};
use common::protocol::{SerializedChunk, Update};
use common::terrain::{ChunkSet, Terrain};
use common::ticks::Ticks;
use common::velocity::Velocity;
use core_protocol::id::PlayerId;
use glam::Vec2;
use std::collections::{HashMap, HashSet};

/// A "Complete" server to client update that references world data to avoid additional allocation.
pub struct CompleteRef<'a, I: Iterator<Item = ContactRef<'a>>> {
    /// Always some, until taken.
    contacts: Option<I>,
    player: AtomicRef<'a, Player>,
    world: &'a World,
    camera_pos: Vec2,
    camera_radius: f32,
}

impl<'a, I: Iterator<Item = ContactRef<'a>>> CompleteRef<'a, I> {
    pub fn new(
        contacts: I,
        player: AtomicRef<'a, Player>,
        world: &'a World,
        camera_pos: Vec2,
        camera_radius: f32,
    ) -> Self {
        Self {
            contacts: Some(contacts),
            player,
            world,
            camera_pos,
            camera_radius,
        }
    }

    pub fn into_update(
        self,
        loaded_entities: &mut HashMap<EntityId, Ticks>,
        loaded_chunks: &mut ChunkSet,
        chunk_loading_cooldown: &mut Ticks,
    ) -> Update {
        let death_reason = if let Status::Dead { reason, .. } = &self.player.status {
            Some(reason.clone())
        } else {
            None
        };

        // Any updated chunks are now no longer loaded.
        *loaded_chunks = loaded_chunks.and(&self.world.terrain.updated.not());

        let loading = if *chunk_loading_cooldown == Ticks::ZERO {
            // Set delay before loading any more chunks.
            *chunk_loading_cooldown = Ticks::from_secs(1.0);

            // All chunks that are currently visible.
            let visible = ChunkSet::new_radius(self.camera_pos, self.camera_radius);

            // Actually load more chunks.
            let ret = visible.and(&loaded_chunks.not());

            // The chunks that will be loaded following this message.
            *loaded_chunks = visible.or(loaded_chunks);

            ret
        } else {
            // Don't load chunks too frequently.
            ChunkSet::new()
        };

        let terrain = loading
            .into_iter()
            .map(|id| SerializedChunk(id, self.world.terrain.get_chunk(id).to_bytes()))
            .collect();

        // For draining loaded_contacts.
        let mut visible_entities = HashSet::new();

        let ret = Update {
            contacts: self
                .contacts
                .unwrap()
                .filter_map(|contact| {
                    visible_entities.insert(contact.id());

                    // Enforce keep alive period by omitting recently sent contacts.
                    let until_next_send =
                        loaded_entities.entry(contact.id()).or_insert(Ticks::ZERO);

                    *until_next_send = until_next_send.saturating_sub(
                        if contact.transform().velocity.abs() > Velocity::from_mps(1.0) {
                            // Send more often if moving.
                            Ticks(3)
                        } else {
                            Ticks::ONE
                        },
                    );

                    return if *until_next_send == Ticks::ZERO {
                        *until_next_send = contact
                            .entity_type()
                            .map(|t| t.data().kind.keep_alive())
                            .unwrap_or(EntityKind::MAX_KEEP_ALIVE);
                        Some(contact.into_contact())
                    } else {
                        None
                    };
                })
                .collect(),
            death_reason,
            player_id: self.player.player_id,
            score: self.player.score,
            world_radius: self.world.radius,
            terrain,
        };
        loaded_entities.retain(|id, _| visible_entities.contains(id));
        ret
    }
}

impl<'a, I: Iterator<Item = ContactRef<'a>>> CompleteTrait<'a> for CompleteRef<'a, I> {
    type Contact = ContactRef<'a>;
    type Iterator = I;

    fn contacts(&mut self) -> Self::Iterator {
        self.contacts.take().unwrap()
    }

    fn collect_contacts(&mut self) -> Vec<Self::Contact> {
        self.contacts.take().unwrap().collect()
    }

    fn death_reason(&self) -> Option<&DeathReason> {
        if let Status::Dead { reason, .. } = &self.player.status {
            Some(reason)
        } else {
            None
        }
    }

    #[inline]
    fn score(&self) -> u32 {
        self.player.score
    }

    #[inline]
    fn world_radius(&self) -> f32 {
        self.world.radius
    }

    #[inline]
    fn terrain(&self) -> &Terrain {
        // TODO limit visibility of terrain.
        &self.world.terrain
    }

    #[inline]
    fn player_id(&self) -> PlayerId {
        self.player.player_id
    }
}
