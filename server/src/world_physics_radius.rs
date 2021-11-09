// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entities::EntityIndex;
use crate::entity::Entity;
use crate::player::Status;
use crate::world::World;
use crate::world_mutation::Mutation;
use arrayvec::ArrayVec;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::*;
use common::ticks::Ticks;
use common::velocity::Velocity;
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use servutil::benchmark::Timer;
use servutil::benchmark_scope;
use std::sync::Arc;
use std::sync::Mutex;

impl World {
    /// minimum_scan_radius returns the radius must be scanned to properly resolve all entity vs.
    /// entity interactions.
    fn minimum_scan_radius(entity: &Entity, delta_seconds: f32) -> f32 {
        let data = entity.data();

        // Enough for collision only.
        let mut radius =
            data.radius * 2.0 + entity.transform.velocity.abs().to_mps() * delta_seconds;

        match data.kind {
            EntityKind::Aircraft | EntityKind::Weapon => {
                // Enough for guidance, deploying sub-armaments, etc.
                radius = radius.max(data.sensors.max_range());
            }
            _ => {}
        }

        radius
    }

    /// parcel_entities adds zero, one, or both entities to an ArrayVec, based on whether they match
    /// a given entity kind.
    fn parcel_entities<'a>(
        entity: &'a Entity,
        other_entity: &'a Entity,
        kind: EntityKind,
    ) -> ArrayVec<&'a Entity, 2> {
        let mut ret = ArrayVec::new();
        if entity.data().kind == kind {
            ret.push(entity);
        }
        if other_entity.data().kind == kind {
            ret.push(other_entity);
        }
        ret
    }

    /// update_entities_and_others performs updates on each pair of entities, with some exceptions.
    pub fn physics_radius(&mut self, delta: Ticks) {
        benchmark_scope!("physics_radius");

        let delta_seconds = delta.to_secs();

        // TODO: look into lock free data structures.
        let mutations = Mutex::new(Vec::new());

        // Call when any entity that is potentially a weapon is removed, to make sure it is reloaded
        // if it is a limited armament. No need to call if the player is definitely not alive.
        let potential_limited_reload = |potentially_limited_entity: &Entity, instant: bool| {
            if !potentially_limited_entity.data().limited {
                // Not actually limited.
                return;
            }
            let player = potentially_limited_entity.borrow_player();
            if let Status::Alive { entity_index, .. } = player.status {
                mutations.lock().unwrap().push((
                    entity_index,
                    Mutation::ReloadLimited {
                        entity_type: potentially_limited_entity.entity_type,
                        instant,
                    },
                ));
            }
        };

        self.entities
            .par_iter()
            .for_each(|(index, entity)| {
                let data = entity.data();

                if data.kind == EntityKind::Collectible {
                    // Collectibles, due to their large number and insignificance, are not worthy
                    // of iterating other entities. Instead, they can be affected by other entities
                    // iterating over them. The side effect is that two collectibles cannot
                    // interact with each other.
                    return; // continue
                }

                let radius = Self::minimum_scan_radius(entity, delta_seconds);

                for (other_index, other_entity) in self.entities.iter_radius(
                    entity.transform.position,
                    radius,
                ) {
                    if index == other_index {
                        // Entities do not interact with themselves.
                        continue;
                    }
                    let _other_data = other_entity.data();

                    // Only want to process each pair of entities once, but without stopping early
                    // and assuming A.radius > B.radius, would process A -> B and, depending on
                    // the exact radii, B -> A. Preventing the second case gives the desired
                    // uniqueness guarantee. For the case of equal radii, the index is used to
                    // pick one permutation to block randomly. Must use scan radius, not entity
                    // radius, such that weapon guidance doesn't get ignored.
                    let other_radius = Self::minimum_scan_radius(other_entity, delta_seconds);
                    #[allow(clippy::float_cmp)]
                    if other_radius > radius || (other_radius == radius && other_index > index) {
                        continue;
                    }

                    let friendly = entity.is_friendly(other_entity);
                    let altitude_overlap = entity.altitude_overlapping(other_entity);

                    let boats = Self::parcel_entities(entity, other_entity, EntityKind::Boat);
                    let mut weapons = Self::parcel_entities(entity, other_entity, EntityKind::Weapon);
                    weapons.extend(
                        Self::parcel_entities(entity, other_entity, EntityKind::Aircraft).into_iter(),
                    );
                    let decoys = Self::parcel_entities(entity, other_entity, EntityKind::Decoy);
                    let collectibles =
                        Self::parcel_entities(entity, other_entity, EntityKind::Collectible);
                    let obstacles = Self::parcel_entities(entity, other_entity, EntityKind::Obstacle);

                    // As collectibles don't iterate other entities, it should be impossible for two
                    // collectibles to interact (and this case isn't handled by the following code).
                    debug_assert!(collectibles.len() < 2);

                    let get_index = |e: &Entity| -> EntityIndex {
                        if e == entity {
                            index
                        } else {
                            debug_assert!(e == other_entity);
                            other_index
                        }
                    };

                    let mutate =
                        |e: &Entity, m: Mutation| mutations.lock().unwrap().push((get_index(e), m));

                    macro_rules! debug_remove {
                        ($entity:expr, $($arg:tt)*) => {
                            #[cfg(debug_assertions)]
                            mutate($entity, Mutation::Remove(DeathReason::Debug(format!($($arg)*))));

                            #[cfg(not(debug_assertions))]
                            mutate($entity, Mutation::Remove(DeathReason::Unknown));
                        }
                    }

                    if !entity.collides_with(other_entity, delta_seconds) || !altitude_overlap {
                        if collectibles.len() == 1 && altitude_overlap {
                            // Collectibles gravitate towards players (except if the player created them).
                            if boats.len() == 1 && (!entity.has_same_player(other_entity) || collectibles[0].ticks > Ticks::from_secs(5.0)) {
                                mutate(collectibles[0], Mutation::Attraction(boats[0].transform.position - collectibles[0].transform.position, Velocity::from_mps(20.0)));
                            }

                            // Payments gravitate towards oil rigs.
                            if obstacles.len() == 1 && obstacles[0].entity_type == EntityType::OilPlatform && collectibles[0].player.is_some() {
                                mutate(collectibles[0], Mutation::Attraction(obstacles[0].transform.position - collectibles[0].transform.position, Velocity::from_mps(10.0)));
                            }
                        }

                        if !friendly {
                            // Mines also gravitate towards boats.
                            if boats.len() == 1 && weapons.len() == 1 && altitude_overlap && weapons[0].data().sub_kind == EntitySubKind::Mine {
                                if weapons[0].is_in_close_proximity_to(&boats[0]) {
                                    mutate(weapons[0], Mutation::Attraction(boats[0].transform.position - weapons[0].transform.position, Velocity::from_mps(5.0)));
                                }
                            }

                            // Make sure to consider case of 2 weapons, a SAM and a missile, not
                            // just the case of a weapon/aircraft and a non-weapon/aircraft target.
                            for weapon in weapons.iter() {
                                // It is easier if the weapon and target are easily accessible.
                                let weapon_data = weapon.data();

                                // Target is the opposite entity.
                                let target = if weapon == &entity { other_entity } else { entity };
                                let target_data = target.data();

                                if weapon_data.sensors.any() {
                                    // Home towards target/decoy
                                    // Sensor activates after 1 second.
                                    if weapons[0].ticks > Ticks::from_secs(1.0) {
                                        let relevant;

                                        match weapon_data.sub_kind {
                                            EntitySubKind::Sam => {
                                                relevant = target_data.kind == EntityKind::Aircraft || target_data.sub_kind == EntitySubKind::Missile || target_data.sub_kind == EntitySubKind::Rocket;
                                            },
                                            EntitySubKind::Torpedo => {
                                                relevant = target_data.kind == EntityKind::Boat || target_data.kind == EntityKind::Decoy;
                                            },
                                            _ => {
                                                relevant = target_data.kind == EntityKind::Boat;
                                            }
                                        }

                                        if relevant {
                                            let diff = target.transform.position - weapon.transform.position;
                                            let angle = Angle::from(diff);

                                            // Should not go off target.
                                            let angle_target_diff = (angle - weapon.guidance.direction_target).abs();
                                            // Cannot sense beyond this angle.
                                            let angle_diff = (angle - weapon.transform.direction).abs();
                                            if angle_target_diff <= Angle::from_degrees(60.0) && angle_diff <= Angle::from_degrees(80.0) {
                                                let mut size = target_data.radius;
                                                if target_data.kind == EntityKind::Decoy {
                                                    // Decoys appear very large to weapons.
                                                    size += 200.0;
                                                } else if target_data.kind == EntityKind::Boat && target_data.sensors.any() && target.extension().is_active() {
                                                    // So do boats with active sensors.
                                                    size += 100.0;
                                                }

                                                let strength = size / EntityData::MAX_RADIUS - diff.length() / radius - angle_diff.to_radians() / Angle::MAX.to_radians();
                                                mutate(weapon, Mutation::Guidance {direction_target: angle, altitude_target: target.altitude, signal_strength: strength});
                                            }
                                        }
                                    }
                                }

                                // Aircraft/ASROC (simulate weapons and anti-aircraft)
                                let asroc = weapon_data.sub_kind == EntitySubKind::Rocket && !weapon_data.armaments.is_empty();
                                if (weapon_data.kind == EntityKind::Aircraft || asroc) && target_data.kind == EntityKind::Boat {
                                    // Small window of opportunity to fire
                                    // Uses lifespan as torpedo consumption
                                    if (weapon.ticks > Ticks::from_secs(3.0 * weapon_data.armaments.len() as f32) || asroc) && weapon.collides_with(target, 1.7+ target_data.length*0.01+weapon.hash()*0.5) {
                                        mutate(weapon, Mutation::FireAll);

                                        if asroc {
                                            // ASROC expires when dropping torpedo.
                                            potential_limited_reload(weapon, false);
                                            debug_remove!(weapon, "asroc");
                                        }
                                    }

                                    // Automatic anti-aircraft has a chance of killing aircraft.
                                    if target_data.anti_aircraft > 0.0 && weapon_data.kind == EntityKind::Aircraft {
                                        let d2 = weapon.transform.position.distance_squared(target.transform.position);
                                        let r2 = (target_data.radius * 1.5).powi(2);

                                        // In range of aa.
                                        if d2 <= r2 {
                                            let chance = (1.0 - d2/r2) * target_data.anti_aircraft;
                                            if thread_rng().gen_bool(chance.clamp(0.0, 1.0) as f64) {
                                                potential_limited_reload(weapon, false);
                                                debug_remove!(weapon, "shot down");
                                            }
                                        }
                                    }
                                }
                            }
                        } else if boats.len() == 1 && weapons.len() == 1 && boats[0].has_same_player(&weapons[0]) &&
                            weapons[0].data().kind == EntityKind::Aircraft &&
                            weapons[0].ticks > Ticks::from_secs(5.0) && weapons[0].can_land_on(&boats[0]) {

                            // Reload instantly since landed.
                            potential_limited_reload(&weapons[0], true);
                            debug_remove!(weapons[0], "landed");
                        }

                        /*
                        if otherData.AntiAircraft != 0 && entityData.Kind == world.EntityKindAircraft {
                            d2 := entity.Position.DistanceSquared(other.Position)
                            r2 := square(otherData.Radius * 1.5)

                            // In range of aa
                            if d2 < r2 {
                                chance := (1.0 - d2/r2) * otherData.AntiAircraft
                                if chance*timeDeltaSeconds > rand.Float32() {
                                    removeEntity(entity, world.DeathReason{})
                                }
                            }
                        }
                         */

                        continue;
                    }

                    #[allow(clippy::if_same_then_else)]
                    if boats.len() == 1 && collectibles.len() == 1 {
                        let is_tanker = boats[0].data().sub_kind == EntitySubKind::Tanker;
                        let score = match collectibles[0].entity_type {
                            EntityType::Barrel => 1 + is_tanker as u32,
                            EntityType::Coin => 10,
                            EntityType::Crate => 2,
                            EntityType::Scrap => 2,
                            _ => 0,
                        };

                        mutate(
                            collectibles[0],
                            Mutation::CollectedBy(
                                Arc::clone(boats[0].player.as_ref().unwrap()),
                                score,
                            ),
                        );

                        if !friendly {
                            mutate(boats[0], Mutation::Repair(Ticks::from_secs(1.0)));
                            mutate(boats[0], Mutation::Reload(collectibles[0].data().reload));
                        }
                    } else if boats.len() == 2 {
                        /*
                            Goals:
                            - (Cancelled) At least one boat is guaranteed to receive fatal damage
                            - Ships with near equal max health and near equal health
                              percentage both die (no seemingly arbitrary survivor)
                            - Low health boats still do damage, hence scale health percent
                        */

                        let base_damage = if friendly {
                            Ticks::ZERO
                        } else {
                            fn damage_contribution(boat: &Entity) -> Ticks {
                                let damage = boat.ticks;
                                let max_health = boat.data().max_health();
                                max_health - damage * 0.5
                            }

                            damage_contribution(entity).min(damage_contribution(other_entity)) * delta / Ticks::RATE
                        };

                        // Process both boats (relative to the other boat).
                        for (boat, other_boat) in [(entity, other_entity), (other_entity, entity)] {
                            // If entity or other_entity appear in this loop, it is almost certainly
                            // an error (only boat and other_boat should be used).

                            // Alias data and other_data to the current iteration.
                            let data = boat.data();
                            let other_data = other_boat.data();

                            // Approximate mass.
                            let mass = data.width * data.length;
                            let other_mass = other_data.width * other_data.length;
                            let mut relative_mass = other_mass / mass;

                            let mut damage = base_damage;

                            if base_damage > Ticks::ZERO {
                                const RAM_DAMAGE_MULTIPLIER: f32 = 3.0;

                                // Colliding with center of boat is more deadly
                                let front_pos = other_boat.transform.position + other_boat.transform.direction.to_vec() * (other_data.length * 0.5);
                                let front_dist2 = front_pos.distance_squared(boat.transform.position);
                                damage *= collision_multiplier(front_dist2, data.radius.powi(2));
                                damage *= boat.extension().spawn_protection();

                                match data.sub_kind {
                                    EntitySubKind::Ram => {
                                        mutate(boat, Mutation::ClearSpawnProtection);
                                        // Reduce recoil.
                                        relative_mass *= 0.5;
                                        // Rams take less damage from ramming.
                                        damage *= 1.0 / RAM_DAMAGE_MULTIPLIER
                                    }
                                    EntitySubKind::Submarine => {
                                        // Subs take more damage from ramming because they are fRaGiLe.
                                        damage *= 2.0
                                    }
                                    _ => ()
                                }

                                match other_data.sub_kind {
                                    EntitySubKind::Ram => {
                                        // Un-reduce recoil.
                                        relative_mass *= 2.0;
                                        // Rams deal more damage while ramming.
                                        damage *= RAM_DAMAGE_MULTIPLIER
                                    }
                                    _ => ()
                                }
                            }

                            let pos_diff = boat.transform.position - other_boat.transform.position;

                            // Closest point to boat's center on other_boat's keel (a line segment from bow to stern).
                            let closest_point_on_other_keel = other_boat.transform.position + pos_diff.project_onto(other_boat.transform.direction.to_vec()).clamp_length_max(other_data.length * 0.5);

                            // Direction of repulsion.
                            let pos_diff_closest_point_on_other_keel = (boat.transform.position - closest_point_on_other_keel).normalize_or_zero();

                            // Velocity change to cause repulsion.
                            let impulse = Velocity::from_mps(6.0 * pos_diff_closest_point_on_other_keel.dot(boat.transform.direction.to_vec()) * relative_mass);

                            mutate(boat, Mutation::CollidedWithBoat{other_player: Arc::clone(other_boat.player.as_ref().unwrap()), damage, ram: other_data.sub_kind == EntitySubKind::Ram, impulse});
                        }
                    } else if boats.len() == 1 && weapons.len() == 1 && !friendly {
                        let dist2 = boats[0]
                            .transform
                            .position
                            .distance_squared(weapons[0].transform.position);
                        let r2 = boats[0].data().radius.powi(2);

                        let damage_resistance = match (boats[0].data().sub_kind, weapons[0].data().sub_kind) {
                            (EntitySubKind::Battleship, EntitySubKind::Torpedo) => 0.6,
                            (EntitySubKind::Cruiser, EntitySubKind::Torpedo) => 0.8,
                            _ => 1.0
                        } * boats[0].extension().spawn_protection();

                        let damage = Ticks::from_damage(
                            weapons[0].data().damage * collision_multiplier(dist2, r2) * damage_resistance,
                        );

                        mutate(
                            boats[0],
                            Mutation::HitBy(
                                Arc::clone(weapons[0].player.as_ref().unwrap()),
                                weapons[0].entity_type,
                                damage,
                            ),
                        );
                        potential_limited_reload(&weapons[0], false);
                        debug_remove!(weapons[0], "hit");
                    } else if boats.len() == 1 && obstacles.len() == 1 {
                        let pos_diff = (boats[0].transform.position - obstacles[0].transform.position).normalize_or_zero();

                        let impulse = Velocity::from_mps(6.0 * pos_diff.dot(boats[0].transform.direction.to_vec())).clamp_magnitude(Velocity::from_mps(30.0));

                        mutate(boats[0], Mutation::CollidedWithObstacle{impulse, entity_type: obstacles[0].entity_type});
                    } else if collectibles.len() == 1
                        && obstacles.len() == 1
                    {
                        // Coins get consumed every other collectible passes under.
                        if obstacles[0].entity_type == EntityType::OilPlatform && collectibles[0].player.is_some() {
                            if rand::thread_rng().gen_bool(0.1) {
                                mutate(obstacles[0], Mutation::UpgradeHq);
                            }

                            debug_remove!(collectibles[0], "consumed");
                        }

                        // Collectibles don't collide with obstacles.
                    } else if boats.len() == 1 && decoys.len() == 1 {
                        // No-op; boats don't collide with decoys.
                    } else if weapons.len() == 1
                        && collectibles.len() == 1
                        && collectibles[0].entity_type == EntityType::Coin
                    {
                        // No-op; don't allow coins (possibly placed by players) to interfere
                        // with enemy weapons.
                    } else if !friendly {
                        // Aside from some edge cases, just remove both entities.
                        for e in [entity, other_entity] {
                            match e.data().kind {
                                EntityKind::Obstacle => {
                                    // Never remove obstacles (at least not here).
                                }
                                EntityKind::Boat => {
                                    panic!("boat's should be removed very intentionally; encountered {:?} -> {:?}", entity.entity_type, other_entity.entity_type);
                                }
                                _ => {
                                    potential_limited_reload(e, false);
                                    debug_remove!(e, "generic");
                                }
                            }
                        }
                    }
                }
            });

        let mut mutations = mutations.into_inner().unwrap();

        // Sort by reverse EntityIndex while prioritizing Mutation ordering.
        mutations.par_sort_unstable_by(|a, b| {
            b.0.cmp(&a.0).then_with(|| {
                b.1.absolute_priority().cmp(&a.1.absolute_priority()).then(
                    b.1.relative_priority()
                        .partial_cmp(&a.1.relative_priority())
                        .unwrap(),
                )
            })
        });

        // Apply mutations (already reversed).
        let mut skip = None;
        for (index, mutation) in mutations {
            if skip != Some(index) && mutation.apply(self, index, delta) {
                skip = Some(index);
            }
        }
    }
}

/// Computes mutlipier for damage such that hits closer to center of boat do more damage.
fn collision_multiplier(d2: f32, r2: f32) -> f32 {
    (0f32.max(r2 - d2 + 90.0) / r2).clamp(0.5, 1.5)
}
