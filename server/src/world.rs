// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::entities::{Entities, EntityIndex};
use crate::entity::Entity;
use crate::noise::noise_generator;
use common::death_reason::DeathReason;
use common::entity::{EntityKind, EntityType};
use common::terrain::Terrain;
use common::ticks::Ticks;
use std::sync::Mutex;

/// A game world of variable radius, consisting of entities and a terrain.
pub struct World {
    pub arena: Arena,
    pub entities: Entities,
    pub terrain: Terrain,
    pub radius: f32,
}

impl World {
    /// new allocates a new World with the given parameters.
    pub fn new(initial_radius: f32) -> Self {
        Self {
            arena: Arena::new(),
            entities: Entities::new(),
            terrain: Terrain::with_generator(noise_generator),
            radius: initial_radius,
        }
    }

    /// update updates the internals of the world, spawning and updating existing entities.
    pub fn update(&mut self, delta: Ticks) {
        self.spawn_statics(delta);
        self.physics(delta);
        self.physics_radius(delta);
        self.arena.recycle();

        let total_visual_area = EntityType::iter()
            .map(|t| {
                let data = t.data();
                if data.kind == EntityKind::Boat {
                    self.arena.count(t) as f32 * data.visual_area()
                } else {
                    0.0
                }
            })
            .sum::<f32>();

        let target_radius = Self::target_radius(total_visual_area);
        let s = delta.to_secs();
        self.radius += (target_radius - self.radius).clamp(s * -0.5, s);
    }

    /// add adds an entity to the world (assigning it an id).
    pub fn add(&mut self, mut entity: Entity) {
        entity.id = self.arena.new_id(entity.entity_type);
        self.entities.add_internal(entity)
    }

    /// remove removes an entity from the world with a given index and death reason.
    pub fn remove(&mut self, index: EntityIndex, reason: DeathReason) {
        let entity = self.entities.remove_internal(index, reason);
        self.arena.drop_entity(entity);
    }

    /// remove_if removes entities that satisfy a given predicate.
    pub fn remove_if<P>(&mut self, predicate: P)
    where
        P: Fn(&Entity) -> bool + Sync,
    {
        let arena = Mutex::new(&mut self.arena);
        self.entities.remove_if_internal(predicate, |entity| {
            arena.lock().unwrap().drop_entity(entity)
        })
    }

    /// area returns the area of the world, based on it's radius.
    pub fn area(&self) -> f32 {
        self.radius.powi(2) * std::f32::consts::PI
    }

    /// target_count returns the target amount of something with a particular density.
    pub fn target_count(&self, density: f32) -> usize {
        (self.area() * density) as usize
    }

    /// target_radius returns the eventual size of the world, assuming it is nudged in the direction
    /// of meeting the target visual overlap.
    pub fn target_radius(total_visual_area: f32) -> f32 {
        (total_visual_area * Self::BOAT_VISUAL_OVERLAP / std::f32::consts::PI)
            .sqrt()
            .clamp(400.0, Self::max_radius())
    }

    pub fn max_radius() -> f32 {
        Entities::max_world_radius().min(Terrain::max_world_radius())
    }
}
