// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::entity::Entity;
use crate::player::Status;
use crate::protocol::*;
use crate::server::Server;
use crate::world::World;
use common::angle::Angle;
use common::entity::*;
use common::protocol::*;
use common::terrain::TerrainMutation;
use common::ticks::Ticks;
use common::util::{level_to_score, score_to_level};
use common::world::{clamp_y_to_strict_area_border, outside_strict_area, ARCTIC};
use common_util::range::map_ranges;
use game_server::player::PlayerTuple;
use glam::Vec2;
use maybe_parallel_iterator::IntoMaybeParallelIterator;
use rand::{thread_rng, Rng};
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

impl CommandTrait for Spawn {
    fn apply(
        &self,
        world: &mut World,
        player_tuple: &Arc<PlayerTuple<Server>>,
    ) -> Result<(), &'static str> {
        let player = player_tuple.borrow_player();

        if player.data.flags.left_game {
            debug_assert!(
                false,
                "should never happen, since messages should not be accepted"
            );
            return Err("cannot spawn after left game");
        }

        if player.data.status.is_alive() {
            return Err("cannot spawn while already alive");
        }

        if !self.entity_type.can_spawn_as(player.score, player.is_bot()) {
            return Err("cannot spawn as given entity type");
        }

        // These initial positions may be overwritten later.
        let mut spawn_position = Vec2::ZERO;
        let mut spawn_radius = 0.8 * world.radius;

        let mut rng = thread_rng();

        if !(player.is_bot() && rng.gen()) {
            // Default to spawning near the center of the world, with more points making you spawn further north.
            let raw_spawn_y = map_ranges(
                score_to_level(player.score) as f32,
                1.5..(EntityData::MAX_BOAT_LEVEL - 1) as f32,
                -0.75 * world.radius..ARCTIC.min(0.75 * world.radius),
                true,
            );
            debug_assert!((-world.radius..=world.radius).contains(&raw_spawn_y));

            // Don't spawn in wrong area.
            let spawn_y = clamp_y_to_strict_area_border(self.entity_type, raw_spawn_y);

            if spawn_y.abs() > world.radius {
                return Err("unable to spawn this type of boat");
            }

            // Solve circle equation.
            let world_half_width_at_spawn_y = (world.radius.powi(2) - spawn_y.powi(2)).sqrt();
            debug_assert!(world_half_width_at_spawn_y <= world.radius);

            // Randomize horizontal a bit. This value will end up in the range
            // [-world_half_width_at_spawn_y / 2, world_half_width_at_spawn_y / 2].
            let spawn_x = (rng.gen::<f32>() - 0.5) * world_half_width_at_spawn_y;

            spawn_position = Vec2::new(spawn_x, spawn_y);
            spawn_radius = world.radius * (1.0 / 3.0);
        }

        debug_assert!(spawn_position.length() <= world.radius);

        /*
        if !player.player_id.is_bot() {
            debug!(
                "player spawning with {} points, with vertical bias {}, near {} r~{}",
                player.score, vertical_bias, spawn_position, spawn_radius
            );
        }
         */

        let exclusion_zone = match &player.data.status {
            // Player is excluded from spawning too close to where another player sunk them, for
            // fairness reasons.
            Status::Dead {
                reason,
                position,
                time,
                ..
            } => {
                // Don't spawn too far away from where you died.
                spawn_position = *position;
                spawn_radius = (0.4 * world.radius).clamp(1200.0, 3000.0).min(world.radius);

                // Don't spawn right where you died either.
                let exclusion_seconds =
                    if player.score > level_to_score(EntityData::MAX_BOAT_LEVEL / 2) {
                        20
                    } else {
                        10
                    };

                if reason.is_due_to_player()
                    && time.elapsed() < Duration::from_secs(exclusion_seconds)
                {
                    Some(*position)
                } else {
                    None
                }
            }
            _ => None,
        };

        if player.team_id().is_some() || player.invitation_accepted().is_some() {
            // TODO: Inefficient to scan all entities; only need to scan all players. Unfortunately,
            // that data is not available here, currently.
            if let Some((_, team_boat)) = world
                .entities
                .par_iter()
                .into_maybe_parallel_iter()
                .find_any(|(_, entity)| {
                    let data = entity.data();
                    if data.kind != EntityKind::Boat {
                        return false;
                    }

                    if let Some(exclusion_zone) = exclusion_zone {
                        if entity.transform.position.distance_squared(exclusion_zone)
                            < 1100f32.powi(2)
                        {
                            return false;
                        }
                    }

                    let is_team_member = player.team_id().is_some()
                        && entity.borrow_player().team_id() == player.team_id();

                    let was_invited_by = player.invitation_accepted().is_some()
                        && entity.borrow_player().player_id
                            == player.invitation_accepted().as_ref().unwrap().player_id;

                    is_team_member || was_invited_by
                })
            {
                spawn_position = team_boat.transform.position;
                spawn_radius = team_boat.data().radius + 25.0;
            }
        }

        drop(player);

        let mut boat = Entity::new(self.entity_type, Some(Arc::clone(player_tuple)));
        boat.transform.position = spawn_position;
        //#[cfg(debug_assertions)]
        //let begin = std::time::Instant::now();
        if world.spawn_here_or_nearby(boat, spawn_radius, exclusion_zone) {
            /*
            #[cfg(debug_assertions)]
            println!(
                "took {:?} to spawn a {:?}",
                begin.elapsed(),
                self.entity_type
            );
             */
            Ok(())
        } else {
            Err("failed to find enough space to spawn")
        }
    }
}

impl CommandTrait for Control {
    fn apply(
        &self,
        world: &mut World,
        player_tuple: &Arc<PlayerTuple<Server>>,
    ) -> Result<(), &'static str> {
        let mut player = player_tuple.borrow_player_mut();

        // Pre-borrow.
        let world_radius = world.radius;

        return if let Status::Alive {
            entity_index,
            aim_target,
            ..
        } = &mut player.data.status
        {
            let entity = &mut world.entities[*entity_index];

            // Movement
            if let Some(guidance) = self.guidance {
                entity.guidance = guidance;
            }
            *aim_target = if let Some(mut aim_target) = self.aim_target {
                sanitize_floats(aim_target.as_mut(), -world_radius * 2.0..world_radius * 2.0)?;
                Some(
                    (aim_target - entity.transform.position)
                        .clamp_length_max(entity.data().sensors.max_range())
                        + entity.transform.position,
                )
            } else {
                None
            };
            let extension = entity.extension_mut();
            extension.set_submerge(self.submerge);
            extension.set_active(self.active);

            drop(player);

            if let Some(fire) = &self.fire {
                fire.apply(world, player_tuple)?;
            }

            if let Some(pay) = &self.pay {
                pay.apply(world, player_tuple)?;
            }

            if let Some(hint) = &self.hint {
                hint.apply(world, player_tuple)?;
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
        player_tuple: &Arc<PlayerTuple<Server>>,
    ) -> Result<(), &'static str> {
        let player = player_tuple.borrow_player();

        return if let Status::Alive {
            entity_index,
            aim_target,
            ..
        } = player.data.status
        {
            // Prevents limited armaments from being invalidated since all limited armaments are destroyed on upgrade.
            if player.data.flags.upgraded {
                return Err("cannot fire right after upgrading");
            }

            let entity = &mut world.entities[entity_index];

            let data = entity.data();

            let index = self.armament_index as usize;
            if index >= data.armaments.len() {
                return Err("armament index out of bounds");
            }

            if entity.extension().reloads[index] != Ticks::ZERO {
                return Err("armament not yet reloaded");
            }

            let armament = &data.armaments[index];
            let armament_entity_data = armament.entity_type.data();

            // Can't fire if boat is a submerged former submarine.
            if entity.altitude.is_submerged()
                && (data.sub_kind != EntitySubKind::Submarine
                    || matches!(armament_entity_data.kind, EntityKind::Aircraft)
                    || matches!(
                        armament_entity_data.sub_kind,
                        EntitySubKind::Shell | EntitySubKind::Sam
                    ))
            {
                return Err("cannot fire while surfacing as a boat");
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

            if armament_entity_data.sub_kind == EntitySubKind::Depositor {
                if let Some(mut target) = aim_target {
                    // Can't deposit in arctic.
                    target.y = target.y.min(ARCTIC - 2.0 * common::terrain::SCALE);

                    // Clamp target is in valid range from depositor or error if too far.
                    const DEPOSITOR_RANGE: f32 = 60.0;
                    let depositor = armament_transform.position;
                    let pos =
                        clamp_to_range(depositor, target, DEPOSITOR_RANGE, DEPOSITOR_RANGE * 2.0)?;

                    world.terrain.modify(TerrainMutation::simple(pos, 60.0));
                } else {
                    return Err("cannot deposit without aim target");
                }
            } else {
                // Fire weapon.
                let player_arc = Arc::clone(player_tuple);

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

                // Some weapons experience random deviation on launch
                let deviation = match armament_entity_data.sub_kind {
                    EntitySubKind::Rocket | EntitySubKind::RocketTorpedo => 0.05,
                    EntitySubKind::Shell => 0.01,
                    _ => 0.03,
                };
                armament_entity.transform.direction += thread_rng().gen::<Angle>() * deviation;

                if !world.spawn_here_or_nearby(armament_entity, 0.0, None) {
                    return Err("failed to fire from current location");
                }
            }

            let entity = &mut world.entities[entity_index];
            entity.consume_armament(index);
            entity.extension_mut().clear_spawn_protection();

            Ok(())
        } else {
            Err("cannot fire while not alive")
        };
    }
}

impl CommandTrait for Pay {
    fn apply(
        &self,
        world: &mut World,
        player_tuple: &Arc<PlayerTuple<Server>>,
    ) -> Result<(), &'static str> {
        let mut player = player_tuple.borrow_player_mut();

        return if let Status::Alive {
            entity_index,
            aim_target: Some(target),
            ..
        } = player.data.status
        {
            let entity = &world.entities[entity_index];

            // Clamp pay to range or error if too far.
            let max_range = entity.data().radii().end;
            let cutoff_range = (max_range * 2.0).min(max_range + 60.0);
            let target =
                clamp_to_range(entity.transform.position, target, max_range, cutoff_range)?;

            let pay = 10; // Value of coin.
            let withdraw = pay * 2; // Payment has 50% efficiency.

            if player.score < level_to_score(entity.data().level) + withdraw {
                return Err("insufficient funds");
            }

            let mut payment = Entity::new(
                EntityType::Coin,
                Some(Arc::clone(entity.player.as_ref().unwrap())),
            );

            payment.transform.position = target;
            payment.altitude = entity.altitude;

            // If payment successfully spawns, withdraw funds.
            if world.spawn_here_or_nearby(payment, 1.0, None) {
                player.score -= withdraw;
            }

            Ok(())
        } else {
            Err("cannot pay while not alive and aiming")
        };
    }
}

impl CommandTrait for Hint {
    fn apply(
        &self,
        _: &mut World,
        player_tuple: &Arc<PlayerTuple<Server>>,
    ) -> Result<(), &'static str> {
        player_tuple.borrow_player_mut().data.hint = Hint {
            aspect: sanitize_float(self.aspect, 0.5..2.0)?,
        };
        Ok(())
    }
}

impl CommandTrait for Upgrade {
    fn apply(
        &self,
        world: &mut World,
        player_tuple: &Arc<PlayerTuple<Server>>,
    ) -> Result<(), &'static str> {
        let mut player = player_tuple.borrow_player_mut();
        let status = &mut player.data.status;

        if let Status::Alive { entity_index, .. } = status {
            let entity = &mut world.entities[*entity_index];
            if !entity
                .entity_type
                .can_upgrade_to(self.entity_type, player.score, player.is_bot())
            {
                return Err("cannot upgrade to provided entity type");
            }

            if outside_strict_area(self.entity_type, entity.transform.position) {
                return Err("cannot upgrade outside the correct area");
            }

            player.data.flags.upgraded = true;

            let below_full_potential = self.entity_type.data().level < score_to_level(player.score);

            drop(player);

            entity.change_entity_type(self.entity_type, &mut world.arena, below_full_potential);

            Ok(())
        } else {
            Err("cannot upgrade while not alive")
        }
    }
}

/// Returns an error if the float isn't finite. Otherwise, clamps it to the provided range.
fn sanitize_float(float: f32, valid: Range<f32>) -> Result<f32, &'static str> {
    if float.is_finite() {
        Ok(float.clamp(valid.start, valid.end))
    } else {
        Err("float not finite")
    }
}

/// Applies sanitize_float to each element.
fn sanitize_floats<'a, F: IntoIterator<Item = &'a mut f32>>(
    floats: F,
    valid: Range<f32>,
) -> Result<(), &'static str> {
    for float in floats {
        *float = sanitize_float(*float, valid.clone())?;
    }
    Ok(())
}

/// Clamps a center -> target vector to `range` and errors if it's length is greater than
/// `cutoff_range`.
fn clamp_to_range(
    center: Vec2,
    target: Vec2,
    range: f32,
    cutoff_range: f32,
) -> Result<Vec2, &'static str> {
    let delta = target - center;
    if delta.length_squared() > cutoff_range.powi(2) {
        Err("outside maximum range")
    } else {
        Ok(center + delta.clamp_length_max(range))
    }
}
