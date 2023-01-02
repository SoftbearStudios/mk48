// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entities::EntityIndex;
use crate::player::{Flags, Status};
use crate::world::World;
use common::altitude::Altitude;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::*;
use common::terrain::TerrainMutation;
use common::ticks::Ticks;
use common::transform::Transform;
use common::velocity::Velocity;
use common::world::{
    clamp_y_to_strict_area_border, outside_strict_area, strict_area_border_normal, ARCTIC,
};
use common_util::range::map_ranges;
use glam::Vec2;
use maybe_parallel_iterator::{IntoMaybeParallelIterator, MaybeParallelSort};
use rand::Rng;
use std::sync::{Arc, Mutex};

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
        let delta_seconds = delta.to_secs();
        let border_radius = self.radius; // Avoids double borrow.
        let border_radius_squared = self.radius.powi(2);
        let terrain = &self.terrain;

        // Collected updates (order doesn't matter).
        let terrain_mutations = Mutex::new(Vec::new());
        let barrel_spawns = Mutex::new(Vec::new());
        let reset_flags = Mutex::new(Vec::new());

        let mut fates: Vec<_> = self
            .entities
            .par_iter_mut()
            .into_maybe_parallel_iter()
            .filter_map(|(index, entity)| {
                let index = index as EntityIndex;
                let data = entity.data();

                if data.lifespan != Ticks::ZERO {
                    entity.ticks = entity.ticks.saturating_add(delta);

                    // Downgrade or die when expired.
                    if entity.ticks > data.lifespan {
                        return if entity.entity_type == EntityType::Hq {
                            if entity.transform.position.y > ARCTIC {
                                // Prevent excessive buildup of HQ's
                                Some((index, Fate::Remove(DeathReason::Unknown)))
                            } else {
                                Some((index, Fate::DowngradeHq))
                            }
                        } else {
                            Some((index, Fate::Remove(DeathReason::Unknown)))
                        };
                    }
                }

                if entity.player.is_some() {
                    let player = entity.borrow_player();

                    // Remove limited entities if player upgrades or is dead.
                    // Remove all of player's entities when player leaves.
                    if (data.limited
                        && (player.data.flags.upgraded || !player.data.status.is_alive()))
                        || player.data.flags.left_game
                    {
                        return Some((index, Fate::Remove(DeathReason::Unknown)));
                    }
                }

                let mut max_speed = data.speed.to_mps();
                let mut repair_eligible = true;

                match data.kind {
                    EntityKind::Aircraft => {
                        let position_diff = if let Status::Alive {
                            aim_target: Some(aim_target),
                            ..
                        } = entity.borrow_player().data.status
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
                                    // Turn in place.
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
                                | EntitySubKind::RocketTorpedo
                                | EntitySubKind::Sam => {
                                    max_speed = EntityData::SURFACING_PROJECTILE_SPEED_LIMIT;

                                    // TODO: As long as ticks govern max range of weapons, prevent
                                    // weapon from timing out while rising to surface by reversing
                                    // delta.
                                    if data.lifespan != Ticks::ZERO
                                        && altitude_change > Altitude::ZERO
                                    {
                                        entity.ticks = entity.ticks.saturating_sub(delta);
                                    }
                                }
                                EntitySubKind::Mine => {
                                    // Delete mines when leaving populated team.
                                    if entity.borrow_player().data.flags.left_populated_team {
                                        return Some((index, Fate::Remove(DeathReason::Unknown)));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    EntityKind::Boat => {
                        entity.apply_altitude_target(
                            terrain,
                            Some(entity.extension().altitude_target()),
                            2.0,
                            delta,
                        );

                        if entity.borrow_player().data.flags != Flags::default() {
                            reset_flags
                                .lock()
                                .unwrap()
                                .push(Arc::clone(entity.player.as_ref().unwrap()));
                        }
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

                let arctic = entity.transform.position.y >= ARCTIC;

                let collision = entity.collides_with_terrain(terrain, delta_seconds);
                if let Some(collision) = collision {
                    // All non-boats die instantly to terrain.
                    if data.kind != EntityKind::Boat {
                        return Some((index, Fate::Remove(DeathReason::Terrain)));
                    }

                    let immune = data.sub_kind == EntitySubKind::Hovercraft
                        || (arctic && data.sub_kind == EntitySubKind::Icebreaker)
                        || (!arctic && data.sub_kind == EntitySubKind::Dredger);

                    entity.transform.velocity = {
                        let Transform {
                            position,
                            direction,
                            mut velocity,
                            ..
                        } = entity.transform;

                        // Move boat away from terrain if it would taking damage.
                        if !immune {
                            let delta =
                                (collision.average_position - position) * (1.0 / data.length);
                            let dot = direction.to_vec().dot(delta);
                            let push = Velocity::from_mps(dot * -150.0);
                            velocity += push;
                        }
                        velocity.clamp_magnitude(Velocity::from_mps(5.0))
                    };

                    if !matches!(
                        data.sub_kind,
                        EntitySubKind::Hovercraft | EntitySubKind::Dredger
                    ) {
                        let is_icebreaker = arctic && data.sub_kind == EntitySubKind::Icebreaker;
                        let max_breakable = if is_icebreaker {
                            Altitude::MAX
                        } else {
                            Altitude(0)
                        };

                        let breakable = Altitude(0)..=max_breakable;

                        if breakable.contains(&collision.max_altitude) {
                            // Break quicker for less points.
                            let amount = if is_icebreaker { -60.0 } else { -20.0 };

                            // Ships break sand and ice they come into contact with.
                            let terrain_mutation = TerrainMutation::conditional(
                                collision.highest_position,
                                amount,
                                breakable,
                            );

                            terrain_mutations
                                .lock()
                                .unwrap()
                                .push((terrain_mutation, is_icebreaker.then_some(index)));
                        }
                    }

                    if !immune {
                        repair_eligible = false;

                        if entity.kill_in(delta, Ticks::from_secs(4.0)) {
                            return Some((index, Fate::Remove(DeathReason::Terrain)));
                        }
                    }
                } else if data.kind == EntityKind::Boat && !arctic {
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

                let outside_border =
                    entity.transform.position.length_squared() > border_radius_squared;
                let outside_area =
                    outside_strict_area(entity.entity_type, entity.transform.position);

                if outside_border || outside_area {
                    repair_eligible = false;
                    let dead = data.kind != EntityKind::Boat
                        || entity.kill_in(delta, Ticks::from_secs(1.0));

                    let position = &mut entity.transform.position;

                    // Normal of border facing inwards.
                    let mut normal = Vec2::ZERO;
                    if outside_border {
                        let n = position.normalize();
                        *position = n * border_radius;
                        normal = -n;
                    }
                    if outside_area {
                        position.y = clamp_y_to_strict_area_border(entity.entity_type, position.y);
                        normal = strict_area_border_normal(entity.entity_type).unwrap()
                    }

                    entity.transform.velocity =
                        Velocity::from_mps(10.0 * normal.dot(entity.transform.direction.to_vec()));

                    // Everything but boats is instantly killed by border
                    if dead {
                        return Some((index, Fate::Remove(DeathReason::Border)));
                    }
                }

                if data.kind == EntityKind::Boat {
                    entity.update_turret_aim(delta_seconds);
                    entity.reload(delta);
                    entity.extension_mut().update_tickers(delta);

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
                        terrain_mutations.lock().unwrap().push((
                            TerrainMutation::simple(entity.transform.position, -17.5),
                            None,
                        ))
                    }
                }

                if index.changed(entity) {
                    Some((index, Fate::MoveSector))
                } else {
                    None
                }
            })
            .collect();

        for (mutation, award_entity_index) in terrain_mutations.into_inner().unwrap() {
            if self.terrain.modify(mutation).unwrap_or(false) {
                if let Some(index) = award_entity_index {
                    // Terrain actually changed, award some points.
                    self.entities[index].borrow_player_mut().score += 1;
                }
            }
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
        fates.maybe_par_sort_unstable_by(|a, b| b.0.cmp(&a.0));

        for (index, fate) in fates {
            match fate {
                Fate::Remove(reason) => {
                    self.remove(index, reason);
                }
                Fate::MoveSector => {
                    self.entities.move_sector(index);
                }
                Fate::DowngradeHq => {
                    let entity = &mut self.entities[index];
                    entity.ticks = Ticks::ZERO;
                    entity.change_entity_type(EntityType::OilPlatform, &mut self.arena, false);
                }
            }
        }

        #[cfg(debug_assertions)]
        self.entities
            .par_iter()
            .into_maybe_parallel_iter()
            .for_each(|(index, entity)| {
                assert!(!index.changed(entity));
            });

        // Clear flags at end so they can be asserted in Mutation::reload_limited_armament.
        for player in reset_flags.into_inner().unwrap() {
            player.borrow_player_mut().data.flags = Flags::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::Entity;
    use crate::world::World;
    use crate::Server;
    use common::entity::{EntityKind, EntityType};
    use common::terrain::Terrain;
    use common::ticks::Ticks;
    use core_protocol::id::PlayerId;
    use game_server::player::{PlayerData, PlayerTuple};
    use std::num::NonZeroU32;
    use std::sync::Arc;

    /// Tests how long each boat takes to recover from (one tick less than) full damage.
    #[test]
    fn repair_rate() {
        let mut world = World::new(10000.0);
        world.terrain = Terrain::new();

        let cases: Vec<_> = EntityType::iter()
            .filter(|t| t.data().kind == EntityKind::Boat)
            .collect();

        let players: Vec<Arc<PlayerTuple<Server>>> = cases
            .iter()
            .enumerate()
            .map(|(i, _)| {
                Arc::new(PlayerTuple::new(PlayerData::new(
                    PlayerId(NonZeroU32::new(i as u32 + 1).unwrap()),
                    None,
                )))
            })
            .collect();

        for (typ, player) in cases.iter().zip(players.iter()) {
            let mut entity = Entity::new(*typ, Some(Arc::clone(&player)));
            entity.damage(entity.data().max_health() - Ticks::ONE);
            //entity.damage(Ticks::from_damage(1.0));
            assert!(
                world.spawn_here_or_nearby(entity, 10000.0, None),
                "could not spawn {:?}",
                typ
            );
        }

        let mut timings: Vec<_> = cases.iter().map(|case| (*case, None)).collect();
        let mut done = 0;

        let mut counter = Ticks::ZERO;
        'outer: loop {
            for (i, (typ, player)) in cases.iter().zip(players.iter()).enumerate() {
                if let Some(entity) = player
                    .borrow_player()
                    .data
                    .status
                    .get_entity_index()
                    .map(|idx| &world.entities[idx])
                {
                    assert_eq!(
                        entity.entity_type, *typ,
                        "expected {:?} not to change type",
                        *typ
                    );

                    if entity.ticks == Ticks::ZERO {
                        if timings[i].1.is_none() {
                            timings[i].1 = Some(counter);
                            done += 1;
                            if done == cases.len() {
                                break 'outer;
                            }
                        }
                    }
                } else {
                    panic!("Expected {:?} to survive", typ);
                }
            }

            world.physics(Ticks::ONE);
            world.physics_radius(Ticks::ONE);
            world.spawn_statics(Ticks::ONE);
            counter += Ticks::ONE;
        }

        timings.sort_by(|(_, a), (_, b)| a.unwrap().cmp(&b.unwrap()));

        for (case, timing) in timings.iter() {
            println!("{:?} {:?}", case, timing.unwrap());
        }
    }
}
