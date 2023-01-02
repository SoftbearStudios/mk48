// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entities::EntityIndex;
use crate::entity::Entity;
use crate::world::World;
use crate::world_mutation::Mutation;
use arrayvec::ArrayVec;
use common::altitude::Altitude;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::*;
use common::ticks;
use common::ticks::Ticks;
use common::util::hash_u32_to_f32;
use common::velocity::Velocity;
use maybe_parallel_iterator::{IntoMaybeParallelIterator, MaybeParallelSort};
use rand::{thread_rng, Rng};
use std::sync::Arc;
use std::sync::Mutex;

pub const MINE_SPEED: f32 = 8.0;

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
        let delta_seconds = delta.to_secs();

        // TODO: look into lock free data structures.
        let mutations = Mutex::new(Vec::new());

        self.entities
            .par_iter()
            .into_maybe_parallel_iter()
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
                                mutate(collectibles[0], Mutation::Attraction(boats[0].transform.position - collectibles[0].transform.position, Velocity::from_mps(20.0), boats[0].altitude - collectibles[0].altitude));
                            }

                            // Payments gravitate towards oil rigs.
                            if obstacles.len() == 1 && obstacles[0].entity_type == EntityType::OilPlatform && collectibles[0].player.is_some() {
                                mutate(collectibles[0], Mutation::Attraction(obstacles[0].transform.position - collectibles[0].transform.position, Velocity::from_mps(10.0), Altitude::ZERO));
                            }
                        }

                        // Repair obstacles near non bots to prevent them from decaying in front of players.
                        if boats.len() == 1 && obstacles.len() == 1 && !boats[0].borrow_player().player_id.is_bot() && obstacles[0].data().lifespan != Ticks::ZERO {
                            // Repair them ten times as fast as they decay.
                            mutate(obstacles[0], Mutation::Repair(delta * 10.0));
                        }

                        if !friendly {
                            // Mines also gravitate towards boats (even submerged subs).
                            if boats.len() == 1 && weapons.len() == 1 && weapons[0].data().sub_kind == EntitySubKind::Mine && weapons[0].is_in_proximity_to(boats[0], Entity::CLOSE_PROXIMITY) {
                                let weapon_position = weapons[0].transform.position;
                                let closest_point = boats[0].closest_point_on_keel_to(weapon_position, 1.0);
                                mutate(weapons[0], Mutation::Attraction(closest_point - weapon_position, Velocity::from_mps(MINE_SPEED), boats[0].altitude - weapons[0].altitude));
                            }

                            // Make sure to consider case of 2 weapons, a SAM and a missile, not
                            // just the case of a weapon/aircraft and a non-weapon/aircraft target.
                            for weapon in weapons.iter() {
                                // It is easier if the weapon and target are easily accessible.
                                let weapon_data = weapon.data();

                                // Target is the opposite entity.
                                let target = if weapon == &entity { other_entity } else { entity };
                                let target_data = target.data();


                                let sub_kind = weapon_data.sub_kind;
                                let is_rocket_torpedo = data.sub_kind == EntitySubKind::RocketTorpedo;
                                let mut rocket_torpedo_sensed = false;

                                if weapon_data.sensors.any() && sub_kind != EntitySubKind::Rocket {
                                    // Home towards target/decoy
                                    // Sensor activates after 1 second.
                                    if weapons[0].ticks > Ticks::from_secs(1.0) {
                                        // Different targets are relevant to each weapon.
                                        let relevant = match weapon_data.sub_kind {
                                            EntitySubKind::Sam => {
                                                target_data.kind == EntityKind::Aircraft || matches!(target_data.sub_kind, EntitySubKind::Missile | EntitySubKind::Rocket | EntitySubKind::RocketTorpedo)
                                            },
                                            EntitySubKind::Torpedo => {
                                                target_data.kind == EntityKind::Boat || target_data.kind == EntityKind::Decoy
                                            },
                                            EntitySubKind::Missile => {
                                                target_data.kind == EntityKind::Boat && weapon.altitude_overlapping(target)
                                            }
                                            _ => {
                                                target_data.kind == EntityKind::Boat
                                            }
                                        };

                                        if relevant {
                                            // Consider a position slightly ahead of the weapon so that
                                            // targets intersecting the weapon don't produce a
                                            // degenerate angle.
                                            let seeker_position = weapon.transform.position + weapon.transform.direction.to_vec() * weapon.transform.velocity.to_mps().max(2.0);
                                            let target_position = target.closest_point_on_keel_to(seeker_position, 0.5);
                                            let diff = target_position - weapon.transform.position;
                                            let distance_squared = diff.length_squared();
                                            let mut angle = Angle::from(diff);

                                            // Should not exceed range.
                                            let remaining_range = weapon.transform.velocity.to_mps() * weapon.data().lifespan.saturating_sub(weapon.ticks).to_secs() + 30.0;
                                            // Should not go off target.
                                            let angle_target_diff = (angle - weapon.guidance.direction_target).abs();
                                            // Cannot sense beyond this angle.
                                            let angle_diff = (angle - weapon.transform.direction).abs();

                                            let (max_angle_target_diff, max_angle_diff) = if weapon.data().sub_kind == EntitySubKind::Missile {
                                                (Angle::from_degrees(30.0), Angle::from_degrees(40.0))
                                            } else {
                                                (Angle::from_degrees(60.0), Angle::from_degrees(80.0))
                                            };
                                            if (is_rocket_torpedo || distance_squared <= remaining_range.powi(2)) && angle_target_diff <= max_angle_target_diff && angle_diff <= max_angle_diff {
                                                if is_rocket_torpedo {
                                                    rocket_torpedo_sensed = true;
                                                } else {
                                                    let mut size = target_data.radius;
                                                    if target_data.kind == EntityKind::Decoy {
                                                        // Decoys appear very large to weapons.
                                                        size += 200.0;
                                                    } else if target_data.kind == EntityKind::Boat && target_data.sensors.any() && target.extension().is_active() {
                                                        // So do boats with active sensors.
                                                        size += 75.0;
                                                    }

                                                    // Switch target from keel to center of boat if it's rotating away.
                                                    let center_diff = weapon.transform.position - target.transform.position;
                                                    let dir = 1f32.copysign(center_diff.dot(target.transform.direction.to_vec()));
                                                    let target_delta_angle = target.guidance.direction_target - target.transform.direction;
                                                    if dir * target_delta_angle.to_degrees() > 5.0 {
                                                        let diff = target.transform.position - weapon.transform.position;
                                                        let a = Angle::from(diff);
                                                        // Don't flip when above and passed center.
                                                        if (a - weapon.transform.direction).abs() < Angle::from_degrees(90.0) {
                                                            angle = a;
                                                        }
                                                    }

                                                    // Altitude diff.
                                                    let altitude_diff = weapon.altitude.difference(target.altitude).to_norm();

                                                    let randomness = hash_u32_to_f32(target.id.get() ^ weapon.id.get());
                                                    let strength = size / EntityData::MAX_RADIUS
                                                        - distance_squared / radius.powi(2)
                                                        - angle_diff.to_radians() / Angle::MAX.to_radians()
                                                        - altitude_diff
                                                        + (1.0 / 3.0) * randomness;
                                                    mutate(weapon, Mutation::Guidance {direction_target: angle, altitude_target: target.altitude, signal_strength: strength});
                                                }
                                            }
                                        }
                                    }
                                }

                                // Aircraft/ASROC (simulate weapons and anti-aircraft).
                                let fire_all_sub_kind = if weapon_data.sub_kind == EntitySubKind::RocketTorpedo && !weapon_data.armaments.is_empty() && target_data.kind == EntityKind::Boat {
                                    Some(weapon_data.armaments[0].entity_type.data().sub_kind)
                                } else if weapon_data.kind == EntityKind::Aircraft {
                                    match target_data.kind {
                                        EntityKind::Boat => {
                                            weapon_data.armaments.iter().map(|a| a.entity_type.data().sub_kind).find(|&s| {
                                                if s == EntitySubKind::Sam {
                                                    return false;
                                                }
                                                if s == EntitySubKind::Missile && target.altitude.is_submerged() {
                                                    return false;
                                                }
                                                true

                                            })
                                        }
                                        EntityKind::Aircraft => {
                                            weapon_data.armaments.iter().map(|a| a.entity_type.data().sub_kind).find(|&s| s == EntitySubKind::Sam)
                                        }
                                        _ => None
                                    }
                                } else {
                                    None
                                };

                                if let Some(sub_kind) = fire_all_sub_kind {
                                    // If more than one weapon is being fired, then it takes proportionally longer to reload.
                                    let amount = weapon_data.armaments.iter().filter(|a| a.entity_type.data().sub_kind == sub_kind).count();

                                    // Small window of opportunity to fire.
                                    let drop_time = match weapon.data().sub_kind {
                                        // Helicopters are slower, need to drop earlier.
                                        EntitySubKind::Heli => 2.5,
                                        // Rocket torpedoes are fast, need to drop later.
                                        EntitySubKind::RocketTorpedo => 1.1,
                                        _ => 1.75
                                    };

                                    // Uses aircraft lifespan as weapon consumption.
                                    // Don't use future collision based firing for rocket torpedoes.
                                    if rocket_torpedo_sensed || (!is_rocket_torpedo && weapon.ticks > Ticks::from_secs(3.0 * amount as f32) && weapon.collides_with(target, drop_time + weapon.hash() * 0.25)) {
                                        mutate(weapon, Mutation::FireAll(sub_kind));

                                        if weapon_data.sub_kind == EntitySubKind::RocketTorpedo {
                                            // ASROC expires when dropping torpedo.
                                            debug_remove!(weapon, "asroc");
                                        }
                                    }
                                }

                                // Automatic anti-aircraft has a chance of killing aircraft.
                                if target_data.anti_aircraft > 0.0 && weapon_data.kind == EntityKind::Aircraft && target_data.kind == EntityKind::Boat {
                                    let d2 = weapon.transform.position.distance_squared(target.transform.position);
                                    let r2 = target_data.anti_aircraft_range().powi(2);

                                    // In range of aa.
                                    if d2 <= r2 {
                                        let chance = (1.0 - d2/r2) * target_data.anti_aircraft * delta.to_secs();
                                        if thread_rng().gen_bool((chance as f64).clamp(0.0, 1.0)) {
                                            debug_remove!(weapon, "shot down");
                                        }
                                    }
                                }
                            }
                        } else if boats.len() == 1 && weapons.len() == 1 && boats[0].has_same_player(weapons[0]) &&
                            weapons[0].data().kind == EntityKind::Aircraft &&
                            weapons[0].ticks > Ticks::from_secs(5.0) {

                            if let Some(pad) = weapons[0].landing_pad(boats[0]) {
                                mutate(weapons[0], Mutation::Remove(DeathReason::Landing(pad)));
                            }
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

                        // Collecting your own coins does not have auxiliary benefits.
                        if !friendly {
                            // Regenerating due to oil rigs is too OP, as it makes ships immune from
                            // submarines.
                            // https://discord.com/channels/847143438939717663/847150517938946058/989645078043693146
                            if collectibles[0].entity_type != EntityType::Barrel {
                                mutate(boats[0], Mutation::Repair(Ticks::from_secs(1.5)));
                            }
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
                            // Boats with more health have more structural integrity, and therefore
                            // deal more damage during a collision.
                            fn damage_contribution(boat: &Entity) -> Ticks {
                                let damage = boat.ticks;
                                let max_health = boat.data().max_health();
                                max_health - damage * 0.5
                            }

                            damage_contribution(entity).min(damage_contribution(other_entity)) * delta / Ticks::FREQUENCY_HZ
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
                                // Colliding with center of boat is more deadly
                                let front_pos = other_boat.transform.position + other_boat.transform.direction.to_vec() * (other_data.length * 0.5);
                                let front_d2 = front_pos.distance_squared(boat.transform.position);
                                damage *= collision_multiplier(front_d2, data.radius.powi(2), data.sub_kind == EntitySubKind::Submarine);
                                damage *= boat.extension().spawn_protection();

                                // Boats that do more ram damage take less recoil.
                                if data.ram_damage != 1.0 {
                                    relative_mass /= data.ram_damage;
                                }

                                match data.sub_kind {
                                    EntitySubKind::Ram => {
                                        mutate(boat, Mutation::ClearSpawnProtection);
                                        // Rams take less recoil.
                                        relative_mass *= 0.1;
                                        // Rams take less damage from ramming.
                                        damage *= 1.0 / data.ram_damage;
                                    }
                                    EntitySubKind::Submarine => {
                                        // Subs take more damage from ramming because they are fRaGiLe.
                                        damage *= 1.5
                                    }
                                    _ => {
                                        // Rising boats take lots more damage because they weren't
                                        // designed for high pressure (upgrading to ram sub is op).
                                        if boat.altitude.is_submerged() {
                                            damage *= 10.0
                                        }
                                    }
                                }

                                damage *= other_data.ram_damage;
                            } else {
                                // Friendly targets are repelled quicker.
                                relative_mass *= 3.0;
                            }

                            let closest_point_on_other_keel = other_boat.closest_point_on_keel_to(boat.transform.position, 1.0);

                            // Direction of repulsion.
                            let pos_diff_closest_point_on_other_keel = (boat.transform.position - closest_point_on_other_keel).normalize_or_zero();

                            // Velocity change to cause repulsion.
                            let impulse = Velocity::from_mps(2.0 * pos_diff_closest_point_on_other_keel.dot(boat.transform.direction.to_vec()) * relative_mass);

                            mutate(boat, Mutation::CollidedWithBoat{other_player: Arc::clone(other_boat.player.as_ref().unwrap()), damage, ram: other_data.ram_damage > 1.0, impulse});
                        }
                    } else if boats.len() == 1 && weapons.len() == 1 && !friendly {
                        let boat_data = boats[0].data();
                        let weapon_data = weapons[0].data();

                        let d2 = boats[0]
                            .transform
                            .position
                            .distance_squared(weapons[0].transform.position);
                        let r2 = boat_data.radius.powi(2);

                        let damage_resistance = boat_data.resistance_to_subkind(weapon_data.sub_kind) * boats[0].extension().spawn_protection();

                        let damage = ticks::from_damage(
                            weapon_data.damage * collision_multiplier(d2, r2, boat_data.sub_kind == EntitySubKind::Submarine) * damage_resistance,
                        );

                        mutate(
                            boats[0],
                            Mutation::HitBy(
                                Arc::clone(weapons[0].player.as_ref().unwrap()),
                                weapons[0].entity_type,
                                damage,
                            ),
                        );
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
                        && (collectibles[0].entity_type == EntityType::Coin || weapons[0].data().sub_kind != EntitySubKind::Torpedo)
                    {
                        // No-op; don't allow coins (possibly placed by players) to interfere
                        // with enemy weapons.
                        // Also all non-torpedo weapons won't hit crates.
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
                                    debug_remove!(e, "generic");
                                }
                            }
                        }
                    }
                }
            });

        let mut mutations = mutations.into_inner().unwrap();

        // Sort by reverse EntityIndex while prioritizing Mutation ordering.
        mutations.maybe_par_sort_unstable_by(|a, b| {
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
        let mut iter = mutations.into_iter().peekable();
        while let Some((index, mutation)) = iter.next() {
            let last_of_mutation_type = iter
                .peek()
                .map(|(next_index, next_mutation)| {
                    *next_index != index
                        || std::mem::discriminant(&mutation)
                            != std::mem::discriminant(next_mutation)
                })
                .unwrap_or(true);
            if skip != Some(index) && mutation.apply(self, index, delta, last_of_mutation_type) {
                skip = Some(index);
            }
        }
    }
}

/// Computes multiplier for damage such that hits closer to center of boat do more damage.
/// Graph comparing old system (red) to new system (sub yellow, boat red): https://www.desmos.com/calculator/crwtc3u4f3
fn collision_multiplier(d2: f32, r2: f32, is_sub: bool) -> f32 {
    let min = match is_sub {
        false => 0.6,
        true => 0.8,
    };
    ((r2 - d2) / r2 * (1.0 - min) + min).clamp(min, 1.0)
}

#[cfg(test)]
mod tests {
    use crate::entity::Entity;
    use crate::world::World;
    use common::entity::EntityType;
    use common::ticks::Ticks;

    #[test]
    fn test_minimum_scan_radius() {
        let mut minimum_scan_radii: Vec<_> = EntityType::iter()
            .map(|entity_type| {
                let entity = Entity::new(entity_type, None);
                let r = World::minimum_scan_radius(&entity, Ticks::ONE.to_secs());

                (entity_type, r)
            })
            .collect();
        minimum_scan_radii.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

        println!("{:?}", minimum_scan_radii);
    }
}
