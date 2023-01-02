// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entities::EntityIndex;
use crate::entity::Entity;
use crate::player::Status;
use crate::server::Server;
use crate::world::World;
use crate::world_physics_radius::MINE_SPEED;
use common::altitude::Altitude;
use common::angle::Angle;
use common::death_reason::DeathReason;
use common::entity::*;
use common::guidance::Guidance;
use common::terrain::TerrainMutation;
use common::ticks::Ticks;
use common::util::*;
use common::velocity::Velocity;
use game_server::player::PlayerTuple;
use glam::Vec2;
use rand::{thread_rng, Rng};
use std::sync::Arc;

/// Serialized mutations, targeted at an indexed entity, ordered by priority.
#[derive(Clone, Debug)]
pub(crate) enum Mutation {
    CollidedWithBoat {
        other_player: Arc<PlayerTuple<Server>>,
        damage: Ticks,
        impulse: Velocity,
        ram: bool,
    },
    CollidedWithObstacle {
        impulse: Velocity,
        entity_type: EntityType,
    },
    ClearSpawnProtection,
    UpgradeHq,
    #[allow(dead_code)]
    Score(u32),
    Remove(DeathReason),
    Repair(Ticks),
    Reload(Ticks),
    // For things that may only be collected once.
    CollectedBy(Arc<PlayerTuple<Server>>, u32),
    HitBy(Arc<PlayerTuple<Server>>, EntityType, Ticks),
    Attraction(Vec2, Velocity, Altitude), // Altitude is a delta.
    Guidance {
        direction_target: Angle,
        altitude_target: Altitude,
        signal_strength: f32,
    },
    FireAll(EntitySubKind),
}

impl Mutation {
    /// absolute_priority returns the priority of this mutation, higher means higher priority (going first).
    pub fn absolute_priority(&self) -> i8 {
        match self {
            Self::FireAll(_) => 127, // so that ASROC can fire before expiring
            Self::Remove(_) => 126,
            Self::HitBy(_, _, _) => 125,
            Self::CollidedWithBoat { .. } => 124,
            Self::CollectedBy(_, _) => 123,
            Self::Attraction(_, _, _) => 101,
            Self::Guidance { .. } => 100,
            _ => 0,
        }
    }

    /// relative_priority returns the priority of this mutation, relative to other mutations of the same absolute priority.
    /// In order for a mutation type to have relative priority relative to other mutations of the same type, it must have a unique absolute priority.
    /// Higher relative priority goes first.
    pub fn relative_priority(&self) -> f32 {
        match self {
            // If you die from two different things simultaneously, prioritize giving another player your points.
            Self::Remove(death_reason) => {
                if death_reason.is_due_to_player() {
                    1.0
                } else {
                    0.0
                }
            }
            // The last guidance (highest signal strength) is the one that will take effect.
            Self::Guidance {
                signal_strength, ..
            } => -signal_strength,
            // Highest damage goes first.
            Self::HitBy(_, _, damage) => damage.to_secs(),
            Self::CollidedWithBoat { damage, .. } => damage.to_secs(),
            // Closest attraction goes last (takes effect).
            Self::Attraction(delta, _, altitude) => {
                // Distance formula without the sqrt.
                // Factor in speed difference between moving and submerging.
                delta.length_squared() + (altitude.to_meters() * (MINE_SPEED / 50.0)).powi(2)
            }
            _ => 0.0,
        }
    }

    /// apply applies the Mutation and returns if the entity was removed.
    /// is_last_of_type is true iff this mutation is the last of its type for this entity index.
    pub fn apply(
        self,
        world: &mut World,
        index: EntityIndex,
        delta: Ticks,
        is_last_of_type: bool,
    ) -> bool {
        let entities = &mut world.entities;
        match self {
            Self::Remove(reason) => {
                #[cfg(debug_assertions)]
                if entities[index].is_boat() {
                    if let DeathReason::Debug(msg) = reason {
                        panic!("boat removed with debug reason {}", msg);
                    }
                }

                world.remove(index, reason);
                return true;
            }
            Self::HitBy(other_player, weapon_type, damage) => {
                let e = &mut entities[index];
                if e.damage(damage) {
                    let killer_alias = {
                        let e_score = e.borrow_player().score;
                        let mut other_player = other_player.borrow_player_mut();
                        other_player.score += kill_score(e_score, other_player.score);
                        let alias = other_player.alias();
                        drop(other_player);
                        alias
                    };

                    world.remove(index, DeathReason::Weapon(killer_alias, weapon_type));
                    return true;
                }
            }
            Self::CollidedWithBoat {
                damage,
                impulse,
                other_player,
                ram,
            } => {
                let entity = &mut entities[index];
                if entity.damage(damage) {
                    let e_score = entity.borrow_player().score;
                    let killer_alias = {
                        let mut other_player = other_player.borrow_player_mut();
                        other_player.score += ram_score(entity.borrow_player().score, e_score);
                        let alias = other_player.alias();
                        drop(other_player);
                        alias
                    };

                    world.remove(
                        index,
                        if ram {
                            DeathReason::Ram(killer_alias)
                        } else {
                            DeathReason::Boat(killer_alias)
                        },
                    );
                    return true;
                }
                entity.transform.velocity =
                    (entity.transform.velocity + impulse).clamp_magnitude(Velocity::from_mps(15.0));
            }
            Self::CollidedWithObstacle {
                impulse,
                entity_type,
            } => {
                let entity = &mut entities[index];
                if entity.kill_in(delta, Ticks::from_secs(6.0)) {
                    world.remove(index, DeathReason::Obstacle(entity_type));
                    return true;
                }
                entity.transform.velocity =
                    (entity.transform.velocity + impulse).clamp_magnitude(Velocity::from_mps(20.0));
            }
            Self::ClearSpawnProtection => entities[index].extension_mut().clear_spawn_protection(),
            Self::UpgradeHq => {
                let entity = &mut entities[index];
                entity.change_entity_type(EntityType::Hq, &mut world.arena, false);
                entity.ticks = Ticks::ZERO;
            }
            Self::Repair(amount) => {
                entities[index].repair(amount);
            }
            Self::Reload(amount) => {
                entities[index].reload(amount);
            }
            Self::Score(score) => {
                entities[index].borrow_player_mut().score += score;
            }
            Self::CollectedBy(player, score) => {
                player.borrow_player_mut().score += score;
                world.remove(index, DeathReason::Unknown);
                return true;
            }
            Self::Guidance {
                direction_target,
                altitude_target,
                ..
            } => {
                // apply_altitude_target is not reversed by another Guidance mutation, so must
                // be sure to only apply one Guidance mutation.
                if is_last_of_type {
                    let entity = &mut entities[index];
                    entity.guidance.direction_target = direction_target;
                    entity.apply_altitude_target(&world.terrain, Some(altitude_target), 5.0, delta);
                }
            }
            Self::Attraction(delta_pos, velocity, delta_altitude) => {
                let entity = &mut entities[index];
                entity.transform.direction = Angle::from(delta_pos);
                entity.transform.velocity = velocity;
                // Same as above (can't do > 1 time).
                if is_last_of_type {
                    entity.altitude += delta_altitude.clamp_magnitude(Altitude::UNIT * 5.0 * delta);
                }
            }
            Self::FireAll(sub_kind) => {
                let entity = &mut entities[index];

                // Reset entity lifespan (because it is actively engaging in battle.
                entity.ticks = Ticks::ZERO;

                let data = entity.data();
                let armament_entities: Vec<Entity> = data
                    .armaments
                    .iter()
                    .enumerate()
                    .filter_map(|(i, armament)| {
                        let armament_data: &EntityData = armament.entity_type.data();
                        if armament_data.sub_kind == sub_kind {
                            let mut armament_entity = Entity::new(
                                armament.entity_type,
                                Some(Arc::clone(entity.player.as_ref().unwrap())),
                            );

                            armament_entity.ticks =
                                armament.entity_type.reduced_lifespan(Ticks::from_secs(
                                    150.0 / armament_data.speed.to_mps().clamp(15.0, 50.0),
                                ));
                            armament_entity.transform =
                                entity.transform + data.armament_transform(&[], i);
                            armament_entity.altitude = entity.altitude;
                            armament_entity.guidance = Guidance {
                                direction_target: entity.transform.direction, // TODO: Randomize
                                velocity_target: armament.entity_type.data().speed,
                            };

                            // Max drop velocity.
                            armament_entity.transform.velocity = armament_entity
                                .transform
                                .velocity
                                .clamp_magnitude(Velocity::from_mps(50.0));

                            Some(armament_entity)
                        } else {
                            None
                        }
                    })
                    .collect();

                // Cannot spawn in loop that borrows entity's guidance.
                for armament_entity in armament_entities {
                    world.spawn_here_or_nearby(armament_entity, 0.0, None);
                }
            }
        };
        false
    }

    /// Called by World::remove.
    pub fn on_world_remove(world: &mut World, index: EntityIndex, reason: &DeathReason) {
        let entity_type = world.entities[index].entity_type;
        let data: &EntityData = entity_type.data();

        if data.kind == EntityKind::Boat {
            // If killed by a player, that player will get the coins. If killed by land or by
            // fleeing combat, score should be converted into coins to prevent destruction of score.
            // DeathReason::Unknown means player left game.
            let score_to_coins = matches!(
                reason,
                DeathReason::Border
                    | DeathReason::Terrain
                    | DeathReason::Unknown
                    | DeathReason::Obstacle(_)
            );

            Self::boat_died(world, index, score_to_coins);
        } else {
            if matches!(reason, DeathReason::Terrain) || data.sub_kind == EntitySubKind::DepthCharge
            {
                Self::maybe_damage_terrain(world, index);
            }

            if data.limited {
                let boat_index = {
                    let entity = &world.entities[index];
                    let player = entity.borrow_player();
                    if let Status::Alive { entity_index, .. } = player.data.status {
                        Some(entity_index)
                    } else {
                        None
                    }
                };

                if let Some(boat_index) = boat_index {
                    // Reload landed aircraft instantly on the correct pad.
                    let landing_pad = if let DeathReason::Landing(pad) = reason {
                        Some(*pad)
                    } else {
                        None
                    };

                    Self::reload_limited_armament(world, boat_index, entity_type, landing_pad)
                }
            }
        }
    }

    /// Called by on_world_remove when a boat dies.
    /// Applies the effect of a boat dying, such as a reduction in the corresponding player's
    /// score and the spawning of loot.
    fn boat_died(world: &mut World, index: EntityIndex, score_to_coins: bool) {
        let entity = &mut world.entities[index];
        let mut player = entity.borrow_player_mut();
        let mut rng = thread_rng();
        let score = player.score;
        player.score = if player.is_bot() {
            // Make sure there are bots in the shallow area.
            respawn_score(player.score).min(level_to_score(rng.gen_range(1..=2)))
        } else {
            respawn_score(player.score)
        };
        drop(player);

        let data = entity.data();
        debug_assert_eq!(data.kind, EntityKind::Boat);

        // Loot is based on the length of the boat.

        let center = entity.transform.position;
        let normal = entity.transform.direction.to_vec();
        let tangent = Vec2::new(-normal.y, normal.x);
        let altitude = entity.altitude;

        for loot_type in entity.entity_type.loot(score, score_to_coins) {
            let mut loot_entity = Entity::new(loot_type, None);

            // Make loot roughly conform to rectangle of ship.
            loot_entity.transform.position = center
                + normal * (rng.gen::<f32>() - 0.5) * data.length
                + tangent * (rng.gen::<f32>() - 0.5) * data.width;
            loot_entity.altitude = altitude;

            // Randomize lifespan a bit to avoid all spawned entities dying at the same time.
            let lifespan = loot_type.data().lifespan;
            if lifespan != Ticks::ZERO {
                loot_entity.ticks += lifespan * (rng.gen::<f32>() * 0.25)
            }

            world.spawn_here_or_nearby(loot_entity, data.radius * 0.15, None);
        }
    }

    /// Called by on_world_remove when a non-boat dies.
    fn maybe_damage_terrain(world: &mut World, entity_index: EntityIndex) {
        let entity = &world.entities[entity_index];
        let data = entity.data();

        // Dying weapons may leave a mark on the terrain.
        if data.kind == EntityKind::Weapon {
            match data.sub_kind {
                EntitySubKind::DepthCharge
                | EntitySubKind::Missile
                | EntitySubKind::Rocket
                | EntitySubKind::Shell
                | EntitySubKind::Torpedo => {
                    // Weapons less than 0.7 damage do 0.7 amount damage/0.7% of the time.
                    // Otherwise terrain wouldn't change since the delta would be too small.
                    const MIN_AMOUNT: f32 = 0.7;
                    let damage = data.damage;
                    let probability = (damage * (1.0 / MIN_AMOUNT)).clamp(0.0, 1.0);
                    let amount = data.damage.max(MIN_AMOUNT);

                    if thread_rng().gen_bool(probability as f64) {
                        // Modify terrain slightly in front of death, to account for finite tick rate.
                        // Should be more correct, on average.
                        let pos = entity.transform.position
                            + (entity.transform.velocity.to_mps() * (Ticks::ONE.to_secs() * 0.5));
                        world.terrain.modify(TerrainMutation::conditional(
                            pos,
                            -20.0 * amount,
                            Altitude(-10)..=Altitude::MAX,
                        ));
                    }
                }
                _ => (),
            }
        }
    }

    /// Called by on_world_remove when a limited armament (weapon, decoy, or aircraft) dies with a
    /// player that is alive.
    fn reload_limited_armament(
        world: &mut World,
        boat_index: EntityIndex,
        entity_type: EntityType,
        landing_pad: Option<usize>,
    ) {
        let armament_data: &EntityData = entity_type.data();

        // Only call this on limited armaments.
        debug_assert!(armament_data.limited);

        // So far these are the only valid types of limited armaments.
        debug_assert!(
            armament_data.kind == EntityKind::Weapon
                || armament_data.kind == EntityKind::Aircraft
                || armament_data.kind == EntityKind::Decoy
        );

        let try_reload = |a: &Armament, c: &mut Ticks| {
            debug_assert_eq!(a.entity_type, entity_type);
            if *c == Ticks::MAX {
                *c = if landing_pad.is_some() {
                    Ticks::ZERO
                } else {
                    a.reload()
                };
                true
            } else {
                false
            }
        };

        let boat = &mut world.entities[boat_index];
        let armaments = &*boat.data().armaments;
        let consumption = boat.extension_mut().reloads_mut();

        if let Some(i) = landing_pad {
            if try_reload(&armaments[i], &mut consumption[i]) {
                return;
            }
        }

        for (a, c) in armaments
            .iter()
            .zip(consumption.iter_mut())
            .filter(|(a, _)| a.entity_type == entity_type)
        {
            if try_reload(a, c) {
                return;
            }
        }

        // Limited armaments must be reloaded except for when players leave or upgrade.
        // TOOD: Fix to allow not being able to reload after respawning.
        /*
        debug_assert!(
            {
                let player = boat.borrow_player();
                let flags = player.flags;
                flags.left_game || flags.upgraded
            },
            "failed to reload limited armament for {:?} {:?}", boat.entity_type, boat.extension().spawn_protection()
        );
         */
    }
}
