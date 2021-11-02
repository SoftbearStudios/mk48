// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::complete_ref::CompleteRef;
use crate::contact_ref::ContactRef;
use crate::player::{PlayerTuple, Status};
use crate::world::World;
use common::entity::{EntityKind, EntitySubKind};
use common::util::*;
use glam::Vec2;

impl World {
    /// get_player_complete gets the complete update for a player, corresponding to everything they
    /// are able to see at the current moment.
    pub fn get_player_complete<'a>(
        &'a self,
        tuple: &'a PlayerTuple,
    ) -> CompleteRef<'a, impl Iterator<Item = ContactRef>> {
        let player = tuple.borrow();
        let player_entity = match &player.status {
            Status::Alive { entity_index, .. } => {
                let entity = &self.entities[*entity_index];
                debug_assert!(entity.is_boat());
                Some(entity)
            }
            _ => None,
        };

        // Players, whether alive or dead, can see other entities based on these parameters.
        let (visual_range, radar_range, sonar_range, position, active, abs_vel) =
            if let Some(entity) = player_entity {
                let data = entity.data();
                let sensors = &data.sensors;

                // Ranges from -1.0 to 1.0 where 0.0 is sea level.
                let norm_altitude = entity.altitude.to_norm();

                let visual_radar_efficacy = map_ranges(norm_altitude, -0.35..0.0, 0.0..1.0, true);

                let visual_range = sensors.visual.range * visual_radar_efficacy;
                let radar_range = sensors.radar.range * visual_radar_efficacy;

                // Sonar works at full effective range as long as it is not airborne.
                let sonar_range = if entity.altitude.is_airborne() {
                    0.0
                } else {
                    sensors.sonar.range
                };

                if player.status.is_alive() {
                    (
                        visual_range,
                        radar_range,
                        sonar_range,
                        entity.transform.position,
                        entity.extension().is_active(),
                        entity.transform.velocity.abs().to_mps(),
                    )
                } else {
                    panic!("player not alive in outbound");
                }
            } else if let Status::Dead {
                position,
                time,
                visual_range,
                ..
            } = player.status
            {
                let elapsed = time.elapsed().as_secs_f32();
                // Fade out visibility over time to save bandwidth.
                let range = map_ranges(elapsed, 10.0..2.0, 0.0..visual_range, true).max(500.0);
                (range, range, range, position, true, 0.0)
            } else {
                (500.0, 500.0, 500.0, Vec2::ZERO, true, 0.0)
            };

        let visual_range_inv = visual_range.powi(-2);
        let radar_range_inv = radar_range.powi(-2);
        let sonar_range_inv = sonar_range.powi(-2);
        let max_range = visual_range.max(radar_range.max(sonar_range));

        let contacts = player_entity
            .into_iter()
            .chain(
                self.entities
                    .iter_radius(position, max_range)
                    .map(|(_, e)| e)
                    .filter(move |e| Some(*e) != player_entity),
            )
            .filter_map(move |entity| {
                // Limit contacts based on visibility.

                let data = entity.data();

                // Variables related to the relationship between the player and the contact.
                let distance_squared = position.distance_squared(entity.transform.position);
                let same_player =
                    entity.player.is_some() && tuple == &**entity.player.as_ref().unwrap();
                let friendly = entity.is_friendly_to_player(Some(tuple));
                let known = same_player || (friendly && distance_squared < 800f32.powi(2));

                // Variables related to detecting the contact.
                let mut visible = false;
                let mut uncertainty = 0f32;
                let altitude = entity.altitude;

                if !known {
                    let inv_size = data.inv_size;
                    let default_ratio = distance_squared * inv_size;
                    uncertainty = 1.0;
                    let entity_abs_vel = entity.transform.velocity.abs().to_mps();

                    if radar_range_inv.is_finite() && !altitude.is_submerged() {
                        let radar_ratio = default_ratio * radar_range_inv;

                        if active {
                            // Active radar can see moving targets easier.
                            uncertainty =
                                uncertainty.min(radar_ratio * 15.0 / (15.0 + entity_abs_vel));
                        }

                        // Always-on passive radar:
                        // Inlined to allow constant propagation and replace div with mul.
                        const BASE_FACTOR: f32 = 25.0;
                        const BASE_EMISSION: f32 = 5.0f32;
                        // let mut emission = BASE_EMISSION;
                        let passive_radar_ratio = if data.kind == EntityKind::Boat {
                            const BOAT_EMISSION: f32 = 5.0;
                            // emission += BOAT_EMISSION;
                            if entity.extension().is_active() && data.sensors.radar.range > 0.0 {
                                // Active radar gives away entity's position.
                                const ACTIVE_EMISSION: f32 = 20.0;
                                // emission += ACTIVE_EMISSION;
                                BASE_FACTOR / (BASE_EMISSION + BOAT_EMISSION + ACTIVE_EMISSION)
                            } else {
                                BASE_FACTOR / (BASE_EMISSION + BOAT_EMISSION)
                            }
                        } else if data.sub_kind == EntitySubKind::Missile {
                            const MISSILE_EMISSION: f32 = 30.0;
                            // emission += MISSILE_EMISSION;
                            BASE_FACTOR / (BASE_EMISSION + MISSILE_EMISSION)
                        } else {
                            BASE_FACTOR / BASE_EMISSION
                        };
                        // let passive_radar_ratio = BASE_FACTOR / emission;

                        uncertainty = uncertainty.min(passive_radar_ratio);
                    }

                    if sonar_range_inv.is_finite() && !altitude.is_airborne() {
                        let mut sonar_ratio = default_ratio * sonar_range_inv;
                        if active {
                            // Active sonar.
                            uncertainty = uncertainty.min(sonar_ratio);
                        }

                        // Beyond this point, sonar_ratio means passive sonar ratio.

                        // Always-on passive sonar:
                        let mut noise = 2f32.max(entity_abs_vel - 4.0);

                        if data.kind == EntityKind::Boat
                            || data.kind == EntityKind::Weapon
                            || data.kind == EntityKind::Decoy
                        {
                            noise *= 2.0;

                            if data.kind != EntityKind::Boat {
                                noise += 100.0;
                            } else if entity.extension().is_active()
                                && data.sensors.sonar.range > 0.0
                            {
                                // Active sonar gives away entity's position.
                                noise += 20.0;
                            }
                        }

                        sonar_ratio /= noise;

                        // Making noise of your own reduces the performance of
                        // passive sonar
                        sonar_ratio *= 20.0 + abs_vel;
                        uncertainty = uncertainty.min(sonar_ratio);
                    }

                    if visual_range_inv.is_finite() {
                        let mut visual_ratio = default_ratio * visual_range_inv;
                        if altitude.is_submerged() {
                            visual_ratio /=
                                map_ranges(altitude.to_norm(), -0.5..1.0, 0.05..0.8, true);
                        }
                        visible = visual_ratio < 1.0;
                        uncertainty = uncertainty.min(visual_ratio);
                    }

                    if data.kind == EntityKind::Weapon
                        && player_entity.is_some()
                        && entity.is_in_close_proximity_to(player_entity.as_ref().unwrap())
                    {
                        // Give players a fighting chance by showing mines that are attracted
                        // towards them.
                        uncertainty = 0.4;
                    }

                    if uncertainty >= 1.0 {
                        // This player has no knowledge of this entity,
                        // so it is not a contact.
                        return None;
                    }
                }

                let has_type = data.kind == EntityKind::Collectible
                    || friendly
                    || uncertainty < 0.5
                    || distance_squared < 100f32.powi(2);

                Some(ContactRef::new(entity, visible, known, has_type))
            });

        CompleteRef::new(contacts, player, self, position, visual_range)
    }
}
