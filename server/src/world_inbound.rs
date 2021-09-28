// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entity::Entity;
use crate::player::Status;
use crate::protocol::*;
use crate::server::SharedData;
use crate::world::World;
use common::angle::Angle;
use common::entity::*;
use common::protocol::*;
use common::ticks::Ticks;
use common::util::level_to_score;
use glam::Vec2;
use rand::{thread_rng, Rng};
use rayon::iter::ParallelIterator;
use std::sync::Arc;
use std::time::Duration;

impl CommandTrait for Spawn {
    fn apply(
        &self,
        world: &mut World,
        shared_data: &mut SharedData,
        bot: bool,
    ) -> Result<(), &'static str> {
        let player = shared_data.player.borrow();

        if player.status.is_alive() {
            return Err("cannot spawn while already alive");
        }

        if !self.entity_type.can_spawn_as(bot) {
            return Err("cannot spawn as given entity type");
        }

        // Default to spawning near the center of the world.
        let mut spawn_position = Vec2::ZERO;
        let mut spawn_radius = world.radius;

        let exclusion_zone = match &player.status {
            // Player is excluded from spawning too close to where another player sunk them, for
            // fairness reasons.
            Status::Dead {
                reason,
                position,
                time,
                ..
            } => {
                if reason.is_due_to_player() && time.elapsed() < Duration::from_secs(10) {
                    Some(*position)
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(team_id) = player.team_id {
            // TODO: Inefficient to scan all entities; only need to scan all players. Unfortunately,
            // that data is not available here, currently.
            if let Some((_, team_boat)) = world.entities.par_iter().find_any(|(_, entity)| {
                let data = entity.data();
                if data.kind == EntityKind::Boat && entity.borrow_player().team_id == Some(team_id)
                {
                    if let Some(exclusion_zone) = exclusion_zone {
                        if entity.transform.position.distance_squared(exclusion_zone)
                            < 1250f32.powi(2)
                        {
                            // Continue.
                            return false;
                        }
                    }

                    return true;
                }
                false
            }) {
                spawn_position = team_boat.transform.position;
                spawn_radius = team_boat.data().radius + 100.0;
            }
        }

        drop(player);

        let mut boat = Entity::new(self.entity_type, Some(Arc::clone(&shared_data.player)));
        boat.transform.position = spawn_position;
        if world.spawn_here_or_nearby(boat, spawn_radius) && !bot {
            return Ok(());
        }
        Err("failed to find enough space to spawn")
    }
}

impl CommandTrait for Control {
    fn apply(
        &self,
        world: &mut World,
        shared_data: &mut SharedData,
        _bot: bool,
    ) -> Result<(), &'static str> {
        let mut player = shared_data.player.borrow_mut();

        return if let Status::Alive {
            entity_index,
            aim_target,
            ..
        } = &mut player.status
        {
            let entity = &mut world.entities[*entity_index];
            if let Some(guidance) = self.guidance {
                entity.guidance = guidance;
            }
            *aim_target = self.aim_target;
            let extension = entity.extension_mut();
            extension.set_active(self.active);
            if let Some(altitude_target) = self.altitude_target {
                extension.altitude_target = altitude_target;
            }
            Ok(())
        } else {
            Err("cannot control while not alive")
        };
    }
}

impl CommandTrait for Fire {
    fn apply(
        &self,
        world: &mut World,
        shared_data: &mut SharedData,
        _bot: bool,
    ) -> Result<(), &'static str> {
        let player = shared_data.player.as_ref().borrow();

        return if let Status::Alive {
            entity_index,
            aim_target,
            ..
        } = player.status
        {
            let entity = &mut world.entities[entity_index];

            let data = entity.data();

            let index = self.index as usize;
            if index >= data.armaments.len() {
                return Err("armament index out of bounds");
            }

            if entity.extension().reloads[index] != Ticks::ZERO {
                return Err("armament not yet reloaded");
            }

            let armament = &data.armaments[index];
            let armament_entity_data = armament.entity_type.data();

            if data.sub_kind == EntitySubKind::Submarine && entity.altitude.is_submerged() {
                // Submerged submarine
                if armament_entity_data.sub_kind == EntitySubKind::Shell
                    || armament_entity_data.sub_kind == EntitySubKind::Sam
                {
                    return Err("cannot fire provided armament while submerged");
                }
            }

            if let Some(turret_index) = armament.turret {
                let turret_angle = entity.extension().turrets[turret_index];
                let turret = &data.turrets[turret_index];

                // The aim may be outside the range but the turret must not be fired if the turret's
                // current angle is outside the range.
                if !turret.within_azimuth(turret_angle) {
                    return Err("invalid turret azimuth");
                }
            }

            let armament_transform =
                entity.transform + data.armament_transform(&entity.extension().turrets, index);

            let mut failed = false;
            if armament_entity_data.sub_kind == EntitySubKind::Depositor {
                // Depositor.
                if self
                    .position_target
                    .distance_squared(armament_transform.position)
                    > 60f32.powi(2)
                {
                    return Err("outside maximum range");
                }
                world.terrain.modify(self.position_target, 60.0);
            } else {
                // Fire weapon.
                let player_arc = Arc::clone(&shared_data.player);

                drop(player);
                let mut armament_entity = Entity::new(armament.entity_type, Some(player_arc));

                armament_entity.transform = armament_transform;
                armament_entity.altitude = entity.altitude;

                let aim_angle = aim_target
                    .map(|aim| Angle::from(aim - armament_entity.transform.position))
                    .unwrap_or(entity.transform.direction);

                armament_entity.guidance.velocity_target = armament_entity_data.speed;
                armament_entity.guidance.direction_target = aim_angle;

                if armament.vertical {
                    // Vertically-launched armaments can be launched in any horizontal direction.
                    armament_entity.transform.direction = armament_entity.guidance.direction_target;
                }

                if armament_entity_data.sub_kind == EntitySubKind::Rocket {
                    // Rockets experience random angle deviations on launch.
                    armament_entity.transform.direction += thread_rng().gen::<Angle>() * 0.05;
                }

                failed |= !world.spawn_here_or_nearby(armament_entity, 0.0);
            }

            if failed {
                Err("failed to fire from current location")
            } else {
                let entity = &mut world.entities[entity_index];
                entity.consume_armament(index);
                entity.extension_mut().clear_spawn_protection();
                Ok(())
            }
        } else {
            Err("cannot fire while not alive")
        };
    }
}

impl CommandTrait for Pay {
    fn apply(
        &self,
        world: &mut World,
        shared_data: &mut SharedData,
        _bot: bool,
    ) -> Result<(), &'static str> {
        let mut player = shared_data.player.as_ref().borrow_mut();

        return if let Status::Alive { entity_index, .. } = player.status {
            let entity = &world.entities[entity_index];

            if self.position.distance_squared(entity.transform.position)
                > entity.data().radii().end.powi(2)
            {
                return Err("position is too far away to pay");
            }

            let pay = 10; // Value of coin.
            let withdraw = pay * 2; // Payment has 50% efficiency.

            if player.score < level_to_score(entity.data().level) + withdraw {
                return Err("insufficient funds");
            }

            let mut payment = Entity::new(
                EntityType::Coin,
                Some(Arc::clone(entity.player.as_ref().unwrap())),
            );

            payment.transform.position = self.position;

            if world.spawn_here_or_nearby(payment, 1.0) {
                // Payment successfully spawned, withdraw funds.
                player.score -= withdraw;
            }

            Ok(())
        } else {
            Err("cannot pay while not alive")
        };
    }
}

impl CommandTrait for Upgrade {
    fn apply(
        &self,
        world: &mut World,
        shared_data: &mut SharedData,
        bot: bool,
    ) -> Result<(), &'static str> {
        let player = shared_data.player.as_ref().borrow_mut();

        if let Status::Alive { entity_index, .. } = player.status {
            let entity = &mut world.entities[entity_index];
            if !entity
                .entity_type
                .can_upgrade_to(self.entity_type, player.score, bot)
            {
                return Err("cannot upgrade to provided entity type");
            }

            drop(player);

            entity.change_entity_type(self.entity_type, &mut world.arena);

            Ok(())
        } else {
            Err("cannot upgrade while not alive")
        }
    }
}
