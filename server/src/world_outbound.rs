// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::complete_ref::CompleteRef;
use crate::contact_ref::ContactRef;
use crate::entity::Entity;
use crate::player::Status;
use crate::server::Server;
use crate::world::World;
use common::entity::{EntityKind, EntitySubKind};
use common::ticks::Ticks;
use common_util::range::{map_ranges, map_ranges_fast};
use game_server::player::PlayerTuple;
use glam::{vec2, Vec2};

impl World {
    /// get_player_complete gets the complete update for a player, corresponding to everything they
    /// are able to see at the current moment.
    pub fn get_player_complete<'a>(
        &'a self,
        tuple: &'a PlayerTuple<Server>,
    ) -> CompleteRef<'a, impl Iterator<Item = ContactRef>> {
        let player = tuple.borrow_player();
        let player_entity = match &player.data.status {
            Status::Alive { entity_index, .. } => {
                let entity = &self.entities[*entity_index];
                debug_assert!(entity.is_boat());
                Some(entity)
            }
            _ => None,
        };

        struct Camera {
            active: bool,
            inner: f32,
            position: Vec2,
            radar: f32,
            sonar: f32,
            speed: f32,
            view: f32,
            visual: f32,
        }

        // Players, whether alive or dead, can see other entities based on these parameters.
        let camera = if let Some(entity) = player_entity {
            let data = entity.data();
            let sensors = &data.sensors;

            // Ranges from -1.0 to 1.0 where 0.0 is sea level.
            let norm_altitude = entity.altitude.to_norm();

            // Radar and visual don't work well under water.
            let visual_radar_efficacy = map_ranges(norm_altitude, -0.35..0.0, 0.0..1.0, true);

            let visual = sensors.visual.range * visual_radar_efficacy;
            let radar = sensors.radar.range * visual_radar_efficacy;

            // Sonar works at full effective range as long as it is not airborne.
            let sonar = if entity.altitude.is_airborne() {
                0.0
            } else {
                sensors.sonar.range
            };

            if player.data.status.is_alive() {
                Camera {
                    active: entity.extension().is_active(),
                    inner: data.radii().start,
                    position: entity.transform.position,
                    radar,
                    sonar,
                    speed: entity.transform.velocity.abs().to_mps(),
                    view: data.camera_range(),
                    visual,
                }
            } else {
                panic!("player not alive in outbound");
            }
        } else if let Status::Dead {
            position,
            time,
            visual_range,
            ..
        } = player.data.status
        {
            let elapsed = time.elapsed().as_secs_f32();
            // Fade out visibility over time to save bandwidth.
            let range = map_ranges(elapsed, 10.0..2.0, 0.0..visual_range, true).max(500.0);
            Camera {
                active: true,
                inner: 0.0,
                position,
                radar: range,
                sonar: range,
                speed: 0.0,
                view: range,
                visual: range,
            }
        } else {
            let range = 500.0;
            Camera {
                active: true,
                inner: 0.0,
                position: Vec2::ZERO,
                radar: range,
                sonar: range,
                speed: 0.0,
                view: range,
                visual: range,
            }
        };

        let visual_range_inv = camera.visual.powi(-2);
        let radar_range_inv = camera.radar.powi(-2);
        let sonar_range_inv = camera.sonar.powi(-2);
        let max_range = camera.visual.max(camera.radar.max(camera.sonar));
        let close_proximity_squared = player_entity.map_or(0.0, |e| {
            (e.entity_type.data().radius + Entity::CLOSE_PROXIMITY).powi(2)
        });
        let inner_circle_squared = camera.inner.powi(2);
        let camera_pos = camera.position;
        let camera_view = camera.view;

        let contacts = player_entity
            .into_iter()
            .chain(
                self.entities
                    .iter_radius(camera.position, max_range)
                    .map(|(_, e)| e)
                    .filter(move |e| Some(*e) != player_entity),
            )
            .filter_map(move |entity| {
                // Limit contacts based on visibility.

                let data = entity.data();

                // Variables related to the relationship between the player and the contact.
                let distance_squared = camera.position.distance_squared(entity.transform.position);
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

                        if camera.active {
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
                        if camera.active {
                            // Active sonar.
                            uncertainty = uncertainty.min(sonar_ratio);
                        }

                        // Beyond this point, sonar_ratio means passive sonar ratio.

                        // Always-on passive sonar:
                        let mut noise = 2f32
                            .max(entity_abs_vel - data.cavitation_speed(entity.altitude).to_mps());

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
                        sonar_ratio *= 20.0 + camera.speed;
                        uncertainty = uncertainty.min(sonar_ratio);
                    }

                    if visual_range_inv.is_finite() {
                        let mut visual_ratio = default_ratio * visual_range_inv;
                        if altitude.is_submerged() {
                            let extra = if data.kind == EntityKind::Boat
                                && entity.extension().reloads.iter().any(|&t| t > Ticks::ZERO)
                            {
                                // A submarine that has fired recently is visible, for practical reasons.
                                0.05
                            } else {
                                0.0
                            };
                            // Don't clamp high because to_norm can't return above 1.0 (high).
                            visual_ratio /= map_ranges_fast(
                                altitude.to_norm(),
                                -0.5..1.0,
                                0.0..0.8,
                                true,
                                false,
                            ) + extra;
                        }
                        visible = visual_ratio < 1.0;
                        uncertainty = uncertainty.min(visual_ratio);
                    }

                    if player_entity.is_some()
                        && data.kind == EntityKind::Weapon
                        && distance_squared < close_proximity_squared // Do faster check first.
                        && entity.is_in_proximity_to(
                            player_entity.as_ref().unwrap(),
                            Entity::CLOSE_PROXIMITY,
                        )
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
                    || distance_squared < inner_circle_squared;

                Some(ContactRef::new(entity, visible, known, has_type))
            });

        // How much more terrain can be sent.
        // 2.0 supports most computer monitors and phones.
        const MAX_ASPECT: f32 = 2.0;

        let aspect = player.data.hint.aspect;
        let camera_width = camera_view * 2.0;
        let camera_dims = vec2(
            camera_width * aspect.clamp(1.0, MAX_ASPECT),
            camera_width * (1.0 / aspect).clamp(1.0, MAX_ASPECT),
        );

        CompleteRef::new(contacts, player, self, camera_pos, camera_dims)
    }
}
