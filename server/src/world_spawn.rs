// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entity::{unset_entity_id, Entity};
use crate::world::World;
use common::altitude::Altitude;
use common::angle::Angle;
use common::entity::*;
use common::guidance::Guidance;
use common::ticks::Ticks;
use common::transform::Transform;
use common::velocity::Velocity;
use glam::Vec2;
use log::warn;
use rand::prelude::ThreadRng;
use rand::Rng;
use servutil::benchmark::Timer;
use servutil::benchmark_scope;

impl World {
    /// Target density of boats (per square meter).
    pub const BOAT_DENSITY: f32 = 1.0 / 400000.0;
    /// Target density of crates (per square meter).
    const CRATE_DENSITY: f32 = 1.0 / 30000.0;
    /// Target density of obstacles (per square meter).
    const OBSTACLE_DENSITY: f32 = 1.0 / 1000000.0;

    /// spawn_here_or_nearby spawns an entity, adjusting it's position and/or rotation until
    /// it can spawn without colliding with world objects.
    ///
    /// If initial_radius is zero, no attempts are made to adjust the entity, so spawning will
    /// fail if the initial conditions are insufficient.
    ///
    /// Returns true if spawning successful, false if failed.
    ///
    /// INVARIANT: Will not affect any entity indices except adding a new one at the end.
    pub fn spawn_here_or_nearby(&mut self, mut entity: Entity, initial_radius: f32) -> bool {
        let retry = initial_radius > 0.0;
        if retry {
            let mut rng = rand::thread_rng();
            let mut radius = initial_radius.max(1.0);
            let center = entity.transform.position;
            let mut threshold = 5f32;

            let mut governor = 0;

            // Always randomize on first iteration
            while entity.transform.position == center || !self.can_spawn(&entity, threshold) {
                // Pick a new position
                let position = gen_radius(&mut rng, radius);
                entity.transform.position = center + position;
                entity.transform.direction = rng.gen();

                radius = (radius * 1.1).min(self.radius * 0.85);
                threshold = 0.15 + threshold * 0.85; // Approaches 1.0

                governor += 1;
                if governor > 128 {
                    // Don't take down the server just because cannot
                    // spawn an entity.
                    break;
                }
            }

            // Without this, some entities would rotate to angle 0 after spawning.
            // TODO: Maybe not within the scope of this function.
            entity.guidance.direction_target = entity.transform.direction;
        }

        let t = entity.entity_type;
        let spawned = self.try_spawn(entity);
        if !spawned {
            warn!("couldn't spawn {:?}", t);
        }
        spawned
    }

    /// try_spawn attempts to spawn an entity at a position and returns if the entity was spawned.
    pub fn try_spawn(&mut self, entity: Entity) -> bool {
        if self.can_spawn(&entity, 1.0) {
            self.add(entity);
            true
        } else {
            false
        }
    }

    // Threshold ranges from [1,infinity), and makes the spawning more picky.
    // e.g. threshold=2 means that twice the normal radius must be clear of obstacles.
    pub fn can_spawn(&self, entity: &Entity, threshold: f32) -> bool {
        if threshold < 1.0 {
            panic!("invalid threshold {}", threshold);
        }

        if entity.transform.position.length_squared() > self.radius.powi(2) {
            // Outside world.
            return false;
        }

        let data = entity.data();

        // Extra space between entities
        let radius = data.radius;
        let max_t = (radius + EntityData::MAX_RADIUS) * threshold;

        match data.kind {
            EntityKind::Decoy | EntityKind::Weapon => {
                for (_, other_entity) in self.entities.iter_radius(entity.transform.position, max_t)
                {
                    if other_entity.data().kind == EntityKind::Obstacle
                        && entity.collides_with(other_entity, 0.0)
                    {
                        // Cannot spawn
                        return false;
                    }
                }
                return !entity.collides_with_terrain(&self.terrain, 0.0);
            }
            EntityKind::Collectible | EntityKind::Aircraft => {
                return !entity.collides_with_terrain(&self.terrain, 0.0);
            }
            EntityKind::Boat => {
                // TODO: Terrain/keel depth check.
            }
            _ => {}
        }

        // Slow, conservative check.
        if self.terrain.land_in_square(
            entity.transform.position,
            (entity.data().radius * 2.0 + 100.0) * threshold,
        ) {
            return false;
        }

        for (_, other_entity) in self.entities.iter_radius(entity.transform.position, max_t) {
            let other_data = other_entity.data();

            if other_data.kind == EntityKind::Collectible {
                // Collectibles don't block spawning.
                continue;
            }

            let t = (radius + other_data.radius) * threshold;
            if entity
                .transform
                .position
                .distance_squared(other_entity.transform.position)
                <= t.powi(2)
            {
                return false;
            }
        }
        true
    }

    /// Spawn basic entities (crates, oil platforms) to maintain their densities.
    pub fn spawn_statics(&mut self, ticks: Ticks) {
        benchmark_scope!("spawn");

        let crate_count = self.arena.count(EntityType::Crate);
        let platform_count =
            self.arena.count(EntityType::OilPlatform) + self.arena.count(EntityType::Hq);

        self.spawn_static_amount(
            EntityType::Crate,
            crate_count,
            self.target_count(Self::CRATE_DENSITY),
            ticks.0 as usize * 150,
        );

        self.spawn_static_amount(
            EntityType::OilPlatform,
            platform_count,
            self.target_count(Self::OBSTACLE_DENSITY),
            ticks.0 as usize * 2,
        );
    }

    /// Spawns a certain amount of basic entities, all throughout the world.
    fn spawn_static_amount(
        &mut self,
        entity_type: EntityType,
        current: usize,
        target: usize,
        rate: usize,
    ) {
        let mut rng = rand::thread_rng();
        let lifespan = entity_type.data().lifespan;

        for _ in 0..target.saturating_sub(current).min(rate) {
            let position = gen_radius(&mut rng, self.radius);
            let direction = rng.gen();

            // Randomize lifespan a bit to avoid all spawned entities dying at the same time.
            let ticks = if lifespan != Ticks::ZERO {
                lifespan * (rng.gen::<f32>() * 0.25)
            } else {
                Ticks::ZERO
            };

            self.spawn_static(entity_type, position, direction, Velocity::ZERO, ticks);
        }
    }

    /// Spawns one basic entity.
    pub fn spawn_static(
        &mut self,
        entity_type: EntityType,
        position: Vec2,
        direction: Angle,
        velocity: Velocity,
        ticks: Ticks,
    ) {
        self.try_spawn(Entity {
            player: None,
            transform: Transform {
                position,
                direction,
                velocity,
            },
            guidance: Guidance {
                velocity_target: Velocity::ZERO,
                direction_target: direction,
            },
            entity_type,
            ticks,
            id: unset_entity_id(),
            altitude: Altitude::ZERO,
        });
    }
}

/// Samples a point from a circle with the given radius.
fn gen_radius(rng: &mut ThreadRng, radius: f32) -> Vec2 {
    rng.gen::<Angle>().to_vec() * (rng.gen::<f32>().sqrt() * radius)
}
