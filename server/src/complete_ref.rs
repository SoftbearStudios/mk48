// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::contact_ref::ContactRef;
use crate::player::Status;
use crate::server::Server;
use crate::world::World;
use atomic_refcell::AtomicRef;
use common::complete::CompleteTrait;
use common::contact::ContactTrait;
use common::death_reason::DeathReason;
use common::protocol::Update;
use common::terrain;
use common::terrain::{ChunkSet, Terrain};
use common::ticks::{Ticks, TicksRepr};
use common::velocity::Velocity;
use game_server::player::PlayerData;
use glam::Vec2;
use std::ops::RangeInclusive;

/// A "Complete" server to client update that references world data to avoid additional allocation.
pub struct CompleteRef<'a, I: Iterator<Item = ContactRef<'a>>> {
    /// Always some, until taken.
    contacts: Option<I>,
    player: AtomicRef<'a, PlayerData<Server>>,
    world: &'a World,
    camera_pos: Vec2,
    camera_dims: Vec2,
}

impl<'a, I: Iterator<Item = ContactRef<'a>>> CompleteRef<'a, I> {
    pub fn new(
        contacts: I,
        player: AtomicRef<'a, PlayerData<Server>>,
        world: &'a World,
        camera_pos: Vec2,
        camera_dims: Vec2,
    ) -> Self {
        Self {
            contacts: Some(contacts),
            player,
            world,
            camera_pos,
            camera_dims,
        }
    }

    pub fn into_update(self, counter: Ticks, loaded_chunks: &mut ChunkSet) -> Update {
        let death_reason = if let Status::Dead { reason, .. } = &self.player.data.status {
            Some(reason.clone())
        } else {
            None
        };

        // Any updated chunks are now no longer loaded.
        let mut new_loaded_chunks = loaded_chunks.and(&self.world.terrain.updated.not());

        // All chunks that are currently visible (on screen).
        // Uses a rect instead of a circle because that is what the client renders,
        // even though it is slightly less realistic.
        let visible = ChunkSet::new_rect(
            self.camera_pos,
            self.camera_dims + Vec2::splat(terrain::SCALE * 2.0),
        );

        // Actually load more chunks.
        let loading = visible.and(&new_loaded_chunks.not());

        // The chunks that will be loaded following this message.
        new_loaded_chunks = visible.or(&new_loaded_chunks);

        let terrain = loading
            .into_iter()
            .map(|id| {
                (
                    id,
                    self.world.terrain.get_chunk(id).to_serialized_chunk(
                        loaded_chunks.contains(id),
                        &self.world.terrain,
                        id,
                    ),
                )
            })
            .collect();

        *loaded_chunks = new_loaded_chunks;

        Update {
            contacts: self
                .contacts
                .unwrap()
                .filter_map(|contact| {
                    let modulus = if let Some(entity_type) = contact.entity_type() {
                        let range: RangeInclusive<Ticks> = entity_type.data().kind.keep_alive();

                        if contact.transform().velocity.abs() > Velocity::from_mps(1.0) {
                            // Send more often if moving.
                            *range.start()
                        } else {
                            *range.end()
                        }
                    } else {
                        Ticks::from_repr(5)
                    };

                    let send = counter
                        .wrapping_add(Ticks::from_repr(contact.id().get() as TicksRepr))
                        % (modulus + Ticks::ONE)
                        == Ticks::ZERO;
                    send.then(|| contact.into_contact())
                })
                .collect(),
            death_reason,
            score: self.player.score,
            world_radius: self.world.radius,
            terrain,
        }
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
        if let Status::Dead { reason, .. } = &self.player.data.status {
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
}
