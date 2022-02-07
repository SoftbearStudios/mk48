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
use common::util::gen_radius;
use common::velocity::Velocity;
use common::world::clamp_y_to_default_area_border;
use glam::Vec2;
use log::{info, warn};
use rand::{thread_rng, Rng};
use server_util::benchmark::Timer;
use server_util::benchmark_scope;

impl World {
    /// Target square meters of world per square meter of player vision.
    pub const BOAT_VISUAL_OVERLAP: f32 = 0.32;
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
    /// An optional filter can return false to block spawning.
    ///
    /// Returns true if spawning successful, false if failed.
    ///
    /// INVARIANT: Will not affect any entity indices except adding a new one at the end.
    pub fn spawn_here_or_nearby(
        &mut self,
        mut entity: Entity,
        initial_radius: f32,
        exclusion_zone: Option<Vec2>,
    ) -> bool {
        let retry = initial_radius > 0.0;
        if retry {
            let mut rng = rand::thread_rng();
            let mut radius = initial_radius.max(1.0);
            let center = entity.transform.position;
            let mut threshold = 6f32;

            let mut governor: u32 = if entity.is_boat() { 128 } else { 8 };

            // Always randomize on first iteration
            while entity.transform.position == center
                || exclusion_zone
                    .map(|ez| {
                        entity.transform.position.distance_squared(ez) < (threshold * 100.0).powi(2)
                    })
                    .unwrap_or(false)
                || !self.can_spawn(&entity, threshold)
            {
                // Pick a new position
                let position = gen_radius(&mut rng, radius);
                entity.transform.position = center + position;
                entity.transform.direction = rng.gen();

                // Clamp boats to correct area.
                if entity.is_boat() {
                    let y = &mut entity.transform.position.y;
                    *y = clamp_y_to_default_area_border(
                        entity.entity_type,
                        *y,
                        entity.entity_type.data().radius,
                    );
                }

                radius = (radius * 1.1).min(self.radius * 0.85);
                threshold = 0.05 + threshold * 0.95; // Approaches 1.0

                governor -= 1;
                if governor == 0 {
                    // Don't take down the server just because cannot
                    // spawn an entity.
                    break;
                }
            }

            // Without this, some entities would rotate to angle 0 after spawning.
            // TODO: Maybe not within the scope of this function.
            entity.guidance.direction_target = entity.transform.direction;

            if entity.data().kind == EntityKind::Boat {
                info!(
                    "Took {} attempts to spawn {:?} (threshold = {}).",
                    128 - governor,
                    entity.entity_type,
                    threshold
                );
            }
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

    /// Threshold ranges from [1,infinity), and makes the spawning more picky.
    /// e.g. threshold=2 means that twice the normal radius must be clear of obstacles.
    /// below threshold=2, obstacles only matter if they actually intersect.
    pub fn can_spawn(&self, entity: &Entity, threshold: f32) -> bool {
        assert!(threshold >= 1.0, "threshold {} is invalid", threshold);

        if entity.transform.position.length_squared() > self.radius.powi(2) {
            // Outside world.
            return false;
        }

        let data = entity.data();

        // Maximum distance over which collision with any other entity is possible.
        let max_collision_radius = data.radius + EntityData::MAX_RADIUS;

        match data.kind {
            EntityKind::Decoy | EntityKind::Weapon => {
                for (_, other_entity) in self
                    .entities
                    .iter_radius(entity.transform.position, max_collision_radius)
                {
                    if other_entity.data().kind == EntityKind::Obstacle
                        && entity.collides_with(other_entity, 0.0)
                    {
                        // Cannot spawn
                        return false;
                    }
                }
                return entity.collides_with_terrain(&self.terrain, 0.0).is_none();
            }
            EntityKind::Collectible | EntityKind::Aircraft => {
                return entity.collides_with_terrain(&self.terrain, 0.0).is_none();
            }
            EntityKind::Boat => {
                // TODO: Terrain/keel depth check.
            }
            _ => {}
        }

        // Slow, conservative check.
        if self.terrain.land_in_square(
            entity.transform.position,
            (entity.data().radius + common::terrain::SCALE) * 2.0 * threshold,
        ) != data.is_land_based()
        {
            return false;
        }

        for (_, other_entity) in self
            .entities
            .iter_radius(entity.transform.position, max_collision_radius * threshold)
        {
            let other_data = other_entity.data();

            if other_data.kind == EntityKind::Collectible {
                // Collectibles don't block spawning.
                continue;
            }

            let distance_squared = entity
                .transform
                .position
                .distance_squared(other_entity.transform.position);
            let collision_distance = data.radius + other_data.radius;
            let safe_distance = collision_distance
                * if entity.is_boat() && other_entity.is_boat() && other_entity.data().level > 2 {
                    threshold
                } else {
                    (threshold * 0.5).max(1.0)
                };
            if distance_squared <= safe_distance.powi(2) {
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
            |_| Some(EntityType::Crate),
            crate_count,
            self.target_count(Self::CRATE_DENSITY),
            ticks.0 as usize * 150,
        );

        self.spawn_static_amount(
            |position| {
                Some(if position.y >= common::world::ARCTIC {
                    EntityType::Hq
                } else if thread_rng().gen_bool(0.25) {
                    EntityType::OilPlatform
                } else {
                    // Fail, to bias against ocean spawns, in favor of arctic.
                    return None;
                })
            },
            platform_count,
            self.target_count(Self::OBSTACLE_DENSITY),
            ticks.0 as usize * 2,
        );
    }

    /// Spawns a certain amount of basic entities, all throughout the world.
    ///
    /// Takes function to get the exact type of entity to spawn, based on the location.
    fn spawn_static_amount(
        &mut self,
        mut get_entity_type: impl FnMut(Vec2) -> Option<EntityType>,
        current: usize,
        target: usize,
        rate: usize,
    ) {
        let mut rng = rand::thread_rng();

        for _ in 0..target.saturating_sub(current).min(rate) {
            let position = gen_radius(&mut rng, self.radius);
            let direction = rng.gen();

            if let Some(entity_type) = get_entity_type(position) {
                let lifespan = entity_type.data().lifespan;

                // Randomize lifespan a bit to avoid all spawned entities dying at the same time.
                let ticks = if lifespan != Ticks::ZERO {
                    lifespan * (rng.gen::<f32>() * 0.25)
                } else {
                    Ticks::ZERO
                };

                self.spawn_static(entity_type, position, direction, Velocity::ZERO, ticks);
            }
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
