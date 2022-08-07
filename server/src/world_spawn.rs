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
use common::world::distance_to_soft_area_border;
use common_util::range::gen_radius;
use glam::Vec2;
use log::{info, warn};
use rand::{thread_rng, Rng};
use std::time::Instant;

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
    /// An optional exclusion zone can block spawning.
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
            let start_time = Instant::now();
            let mut rng = rand::thread_rng();
            let mut radius = initial_radius.max(1.0);
            let center = entity.transform.position;
            let (max_attempts, mut threshold): (u32, f32) = if entity.is_boat() {
                if entity.borrow_player().player_id.is_bot() {
                    (128, 4.0)
                } else {
                    (1024, 6.0)
                }
            } else {
                (8, 4.0)
            };

            let mut governor = max_attempts;
            let max_distance_from_center =
                (self.radius * 0.9 - entity.data().radius * 1.5).max(self.radius * 0.5);

            // Always randomize on first iteration
            while entity.transform.position == center
                || exclusion_zone
                    .map(|ez| {
                        entity.transform.position.distance_squared(ez)
                            < (threshold.min(3.0) * 500.0).powi(2)
                    })
                    .unwrap_or(false)
                || !self.can_spawn(&entity, threshold, max_distance_from_center)
            {
                // Pick a new position
                let position = gen_radius(&mut rng, radius);
                entity.transform.position = center + position;
                entity.transform.direction = rng.gen();

                radius = (radius * 1.05).min(max_distance_from_center);
                threshold = 0.005 + threshold * 0.995; // Approaches 1.0

                debug_assert!(threshold >= 1.0, "so try_spawn works");

                governor -= 1;
                if governor == 0 {
                    // Don't take down the server just because cannot
                    // spawn an entity.
                    break;
                }
            }

            // Ensure determinism and allowing spawn with threshold 1.0.
            #[cfg(debug_assertions)]
            if governor > 0 {
                for i in 0..3 {
                    debug_assert!(
                        self.can_spawn(&entity, threshold, max_distance_from_center),
                        "i: {}, t: {}",
                        i,
                        threshold
                    );
                }
                for i in 0..3 {
                    debug_assert!(
                        self.can_spawn(&entity, 1.0, max_distance_from_center),
                        "i: {}, t': {}",
                        i,
                        threshold
                    );
                }
            }

            // Without this, some entities would rotate to angle 0 after spawning.
            // TODO: Maybe not within the scope of this function.
            entity.guidance.direction_target = entity.transform.direction;

            if entity.data().kind == EntityKind::Boat {
                info!(
                    "Took {}/{} attempts ({:?}) to spawn {:?} (threshold = {}, final_radius = {}).",
                    max_attempts - governor,
                    max_attempts,
                    start_time.elapsed(),
                    entity.entity_type,
                    threshold,
                    radius,
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
        if self.can_spawn(&entity, 1.0, self.radius) {
            self.add(entity);
            true
        } else {
            false
        }
    }

    /// Threshold ranges from [1,infinity), and makes the spawning more picky.
    /// e.g. threshold=2 means that twice the normal radius must be clear of obstacles.
    /// below threshold=2, obstacles only matter if they actually intersect.
    pub fn can_spawn(
        &self,
        entity: &Entity,
        threshold: f32,
        max_distance_from_center: f32,
    ) -> bool {
        assert!(threshold >= 1.0, "threshold {} is invalid", threshold);
        debug_assert!(
            max_distance_from_center >= 0.0 && max_distance_from_center <= self.radius,
            "max_distance_from_center={:?} radius={:?} is invalid",
            max_distance_from_center,
            self.radius
        );

        if entity.transform.position.length_squared() > max_distance_from_center.powi(2) {
            // Outside world/max radius from center.
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
                return entity
                    .collides_with_terrain(&self.terrain, Ticks::PERIOD_SECS)
                    .is_none();
            }
            EntityKind::Collectible | EntityKind::Aircraft => {
                return entity.collides_with_terrain(&self.terrain, 0.0).is_none();
            }
            EntityKind::Boat => {
                // Reject boats spawning in the wrong area. Don't clamp, as that biases towards
                // spawning on border!
                if distance_to_soft_area_border(entity.entity_type, entity.transform.position)
                    < data.radius + 50.0 * threshold
                {
                    return false;
                }

                // TODO: Terrain/keel depth check.
            }
            _ => {}
        }

        // Slow, conservative check.
        if self.terrain.land_in_square(
            entity.transform.position,
            (entity.data().radius * threshold.min(2.0) + common::terrain::SCALE) * 2.0,
            //if threshold > 3.0 { default_land.min(-entity.data().draft) } else { default_land },
        ) {
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
                * if entity.is_friendly(other_entity) {
                    1.0
                } else if entity.is_boat()
                    && other_entity.is_boat()
                    && data.level.abs_diff(other_data.level) > 1
                {
                    // Be extra careful when there is a level differential.
                    threshold * 1.5
                } else if other_data.kind == EntityKind::Obstacle {
                    threshold.min(1.5)
                } else {
                    // Low level ships and weapons.
                    threshold
                };
            if distance_squared <= safe_distance.powi(2) {
                return false;
            }
        }
        true
    }

    /// Spawn basic entities (crates, oil platforms) to maintain their densities.
    pub fn spawn_statics(&mut self, ticks: Ticks) {
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
                Some(if position.y >= common::world::ARCTIC + 300.0 {
                    EntityType::Hq
                } else if position.y < common::world::ARCTIC && thread_rng().gen_bool(0.2) {
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
