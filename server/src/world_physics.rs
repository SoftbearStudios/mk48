// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entities::EntityIndex;
use crate::entity::Entity;
use crate::player::Status;
use crate::world::World;
use crate::world_mutation::Mutation;
use common::altitude::Altitude;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::*;
use common::ticks::Ticks;
use common::util::map_ranges;
use common::velocity::Velocity;
use glam::Vec2;
use rand::Rng;
use rayon::prelude::*;
use servutil::benchmark::Timer;
use servutil::benchmark_scope;
use std::sync::Mutex;

/// Fate terminates the physics for a particular entity with a single fate.
enum Fate {
    Remove(DeathReason),
    MoveSector,
    DowngradeHq,
}

impl World {
    /// update_entities performs updates intrinsic to one entity (and updates the world radius based
    /// on the number of boats). This is currently the only safe location for entity positions to change, due
    /// to the implementation of `Entities`.
    pub fn physics(&mut self, delta: Ticks) {
        benchmark_scope!("physics");

        let delta_seconds = delta.to_secs();
        let border_radius = self.radius; // Avoids double borrow.
        let border_radius_squared = self.radius.powi(2);
        let terrain = &self.terrain;

        // Collected updates (order doesn't matter).
        let limited_reloads = Mutex::new(Vec::new()); // Of form (player_entity_index, limited_entity_type).
        let terrain_mutations = Mutex::new(Vec::new());
        let barrel_spawns = Mutex::new(Vec::new());

        // Call when any entity that is potentially a weapon is removed, to make sure it is reloaded
        // if it is a limited armament. No need to call if the player is definitely not alive.
        let potential_limited_reload = |potentially_limited_entity: &Entity| {
            if !potentially_limited_entity.data().limited {
                // Not actually limited.
                return;
            }
            let player = potentially_limited_entity.borrow_player();
            if let Status::Alive { entity_index, .. } = player.status {
                limited_reloads
                    .lock()
                    .unwrap()
                    .push((entity_index, potentially_limited_entity.entity_type));
            }
        };

        let mut fates: Vec<_> = self
            .entities
            .par_iter_mut()
            .filter_map(|(index, entity)| {
                let index = index as EntityIndex;
                let data = entity.data();

                if data.lifespan != Ticks::ZERO {
                    entity.ticks += delta; // TODO: potential overflow?

                    // Downgrade or die when expired.
                    if entity.ticks > data.lifespan {
                        return if entity.entity_type == EntityType::Hq {
                            Some((index, Fate::DowngradeHq))
                        } else {
                            potential_limited_reload(entity);
                            Some((index, Fate::Remove(DeathReason::Unknown)))
                        };
                    }
                }

                if data.limited && !entity.borrow_player().status.is_alive() {
                    // Player is definitely not alive, impossible to reload limited armament.
                    return Some((index, Fate::Remove(DeathReason::Unknown)));
                }

                let mut max_speed = data.speed.to_mps();
                let mut repair_eligible = true;

                match data.kind {
                    EntityKind::Aircraft => {
                        let position_diff = if let Status::Alive {
                            aim_target: Some(aim_target),
                            ..
                        } = entity.borrow_player().status
                        {
                            aim_target - entity.transform.position
                        } else {
                            // Hover when no target or player is dead.
                            Vec2::ZERO
                        };

                        entity.guidance.direction_target = Angle::from(position_diff)
                            + Angle::from_radians(
                                (entity.hash() - 0.5) * std::f32::consts::PI * 0.25,
                            );
                        let distance_squared = position_diff.length_squared();

                        let angle_deviation =
                            (entity.transform.direction - entity.guidance.direction_target).abs();

                        match data.sub_kind {
                            EntitySubKind::Heli => {
                                if angle_deviation < Angle::from_degrees(80.0) {
                                    max_speed *= map_ranges(
                                        distance_squared,
                                        5.0..80f32.powi(2),
                                        0.0..1.0,
                                        true,
                                    );
                                } else {
                                    max_speed = 0.0;
                                }
                            }
                            EntitySubKind::Plane => {
                                if distance_squared < 50.0f32.powi(2)
                                    && angle_deviation > Angle::from_degrees(30.0)
                                {
                                    max_speed = max_speed.min(30.0);
                                }
                            }
                            _ => unreachable!(),
                        }

                        entity.apply_altitude_target(terrain, None, 4.0, delta);
                    }
                    EntityKind::Collectible | EntityKind::Weapon | EntityKind::Decoy => {
                        let altitude_change =
                            entity.apply_altitude_target(terrain, None, 3.0, delta);
                        if entity.altitude.is_submerged() {
                            match data.sub_kind {
                                // Wait until risen to surface.
                                EntitySubKind::Missile
                                | EntitySubKind::Rocket
                                | EntitySubKind::Sam => {
                                    max_speed = EntityData::SURFACING_PROJECTILE_SPEED_LIMIT;

                                    // TODO: As long as ticks govern max range of weapons, prevent
                                    // weapon from timing out while rising to surface by reversing
                                    // delta.
                                    if data.lifespan != Ticks::ZERO
                                        && altitude_change > Altitude::ZERO
                                    {
                                        entity.ticks -= delta;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    EntityKind::Boat => {
                        entity.apply_altitude_target(
                            terrain,
                            Some(entity.extension().altitude_target),
                            2.0,
                            delta,
                        );
                    }
                    EntityKind::Obstacle => {
                        let rate: f32 = match entity.entity_type {
                            EntityType::OilPlatform => 1.0 / 4.0,
                            EntityType::Hq => 2.0 / 4.0,
                            _ => 0.0,
                        };

                        if rand::thread_rng()
                            .gen_bool((1.0 - (1.0 - rate).powf(delta_seconds)) as f64)
                        {
                            barrel_spawns
                                .lock()
                                .unwrap()
                                .push(entity.transform.position)
                        }
                    }
                    _ => {}
                }

                entity
                    .transform
                    .apply_guidance(data, entity.guidance, max_speed, delta_seconds);
                entity.transform.do_kinematics(delta_seconds);

                // Collide with terrain.
                if entity.collides_with_terrain(terrain, delta_seconds) {
                    if data.kind != EntityKind::Boat {
                        potential_limited_reload(entity);
                        return Some((index, Fate::Remove(DeathReason::Terrain)));
                    }

                    repair_eligible = false;
                    entity.transform.velocity = entity
                        .transform
                        .velocity
                        .clamp_magnitude(Velocity::from_mps(5.0));

                    if (!(data.sub_kind == EntitySubKind::Dredger
                        || data.sub_kind == EntitySubKind::Hovercraft))
                        && entity.kill_in(delta, Ticks::from_secs(4.0))
                    {
                        return Some((index, Fate::Remove(DeathReason::Terrain)));
                    }
                } else if data.kind == EntityKind::Boat {
                    let below_keel = entity.altitude
                        - terrain
                            .sample(entity.transform.position)
                            .unwrap_or(Altitude::MIN)
                        - data.draft;

                    /*
                    println!("{} -> {:?}         ({:?})", terrain
                        .sample(entity.transform.position).map(|a| a.0).unwrap(), terrain
                                 .sample(entity.transform.position).unwrap(), below_keel);
                     */

                    if below_keel < Altitude::ZERO {
                        repair_eligible = false;
                        let speed_factor =
                            map_ranges(below_keel.to_meters(), -5.0..0.0, 0.6..1.0, true);

                        entity.transform.velocity = entity
                            .transform
                            .velocity
                            .clamp_magnitude(Velocity::from_mps(max_speed * speed_factor));
                    }
                }

                let center_dist2 = entity.transform.position.length_squared();
                if center_dist2 > border_radius_squared {
                    repair_eligible = false;
                    let dead = data.kind != EntityKind::Boat
                        || entity.kill_in(delta, Ticks::from_secs(1.0));
                    entity.transform.position =
                        entity.transform.position.normalize() * border_radius;
                    entity.transform.velocity = Velocity::from_mps(
                        -10.0
                            * entity
                                .transform
                                .position
                                .normalize()
                                .dot(entity.transform.direction.to_vec()),
                    );
                    // Everything but boats is instantly killed by border
                    if dead || center_dist2 > border_radius_squared * 1.1 {
                        potential_limited_reload(entity);
                        return Some((index, Fate::Remove(DeathReason::Border)));
                    }
                }

                if data.kind == EntityKind::Boat {
                    entity.update_turret_aim(delta_seconds);
                    entity.reload(delta);
                    entity
                        .extension_mut()
                        .update_active_cooldown_and_spawn_protection(delta);

                    if repair_eligible {
                        let repair_amount = if data.length > 200.0 {
                            3.0
                        } else if data.length > 100.0 {
                            2.0
                        } else {
                            1.0
                        };
                        entity.repair(delta * repair_amount);
                    }

                    if data.sub_kind == EntitySubKind::Dredger {
                        // Dredgers excavate land they come into contact with.
                        terrain_mutations
                            .lock()
                            .unwrap()
                            .push((entity.transform.position, -5.0));
                    }
                }

                if index.changed(entity) {
                    Some((index, Fate::MoveSector))
                } else {
                    None
                }
            })
            .collect();

        // Must do before removing any entities (and invalidating indices).
        for (player_entity_index, limited_entity_type) in limited_reloads.into_inner().unwrap() {
            Mutation::reload_limited_armament(
                self,
                player_entity_index,
                limited_entity_type,
                false,
            );
        }

        for (pos, amount) in terrain_mutations.into_inner().unwrap() {
            self.terrain.modify(pos, amount);
        }

        // Spawn barrels around oil platforms.
        let mut rng = rand::thread_rng();
        for mut position in barrel_spawns.into_inner().unwrap() {
            const BARREL_RADIUS: f32 = 120.0;
            position +=
                rng.gen::<Angle>().to_vec() * rng.gen_range((BARREL_RADIUS / 2.0)..BARREL_RADIUS);
            let direction = rng.gen();
            let velocity = Velocity::from_mps(rng.gen_range(10.0..20.0));
            self.spawn_static(
                EntityType::Barrel,
                position,
                direction,
                velocity,
                Ticks::ZERO,
            );
        }

        // Sorted in reverse to remove correctly.
        fates.par_sort_unstable_by(|a, b| b.0.cmp(&a.0));

        for (index, fate) in fates {
            match fate {
                Fate::Remove(reason) => {
                    let entity = &self.entities[index];
                    let data = entity.data();
                    match data.kind {
                        EntityKind::Boat => {
                            Mutation::boat_died(self, index);
                        }
                        EntityKind::Weapon => {
                            // Dying weapons may leave a mark on the terrain.
                            match data.sub_kind {
                                EntitySubKind::Torpedo
                                | EntitySubKind::Missile
                                | EntitySubKind::Shell
                                | EntitySubKind::Rocket => {
                                    if rng.gen_bool(data.damage.clamp(0.0, 1.0) as f64) {
                                        // Modify terrain slightly in front of death, to account for finite tick rate.
                                        // Should be more correct, on average.
                                        let pos = entity.transform.position
                                            + (entity.transform.velocity.to_mps()
                                                * delta_seconds
                                                * 0.5);
                                        self.terrain.modify(pos, -8.0 * data.damage);
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                    self.remove(index, reason);
                }
                Fate::MoveSector => {
                    self.entities.move_sector(index);
                }
                Fate::DowngradeHq => {
                    self.entities[index].change_entity_type(EntityType::Hq, &mut self.arena);
                }
            }
        }

        #[cfg(debug_assertions)]
        self.entities.par_iter().for_each(|(index, entity)| {
            assert!(!index.changed(entity));
        });
    }
}
