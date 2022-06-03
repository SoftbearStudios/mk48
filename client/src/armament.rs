// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use crate::interpolated_contact::InterpolatedContact;
use client_util::context::CoreState;
use client_util::rate_limiter::RateLimiter;
use client_util::renderer::particle::{Particle, ParticleLayer};
use common::angle::Angle;
use common::contact::{Contact, ContactTrait};
use common::entity::{EntityData, EntityId, EntityKind, EntitySubKind};
use common_util::range::gen_radius;
use glam::Vec2;
use rand::{thread_rng, Rng};
use std::collections::HashMap;

impl Mk48Game {
    /// Finds the best armament (i.e. the one that will be fired if the mouse is clicked).
    /// Armaments are scored by a combination of distance and angle to target.
    pub fn find_best_armament(
        &self,
        player_contact: &Contact,
        angle_limit: bool,
        mouse_position: Vec2,
        armament_selection: Option<(EntityKind, EntitySubKind)>,
    ) -> Option<usize> {
        // The f32 represents how good the shot is, lower is better.
        let mut best_armament: Option<(usize, f32)> = None;

        if let Some(armament_selection) = armament_selection {
            for i in 0..player_contact.data().armaments.len() {
                let armament = &player_contact.data().armaments[i];

                let armament_entity_data: &EntityData = armament.entity_type.data();

                if !(armament_entity_data.kind == armament_selection.0
                    && armament_entity_data.sub_kind == armament_selection.1)
                {
                    // Wrong type; cannot fire.
                    continue;
                }

                // Don't limit dredger fire rate so players with bad ping can build faster.
                // TODO fix ping reducing fire rate for all weapons.
                if !((player_contact.reloads()[i] && self.fire_rate_limiter.is_ready(i as u8))
                    || armament_entity_data.sub_kind == EntitySubKind::Depositor)
                {
                    // Recently fired, shouldn't try to fire again (server will just block).
                    continue;
                }

                let mut max_angle_diff = Angle::ZERO;
                if let Some(turret_index) = armament.turret {
                    if !player_contact.data().turrets[turret_index]
                        .within_azimuth(player_contact.turrets()[turret_index])
                    {
                        // Out of azimuth range; cannot fire.
                        continue;
                    }
                } else {
                    max_angle_diff += Angle::from_degrees(30.0)
                }

                let transform = *player_contact.transform()
                    + player_contact
                        .data()
                        .armament_transform(player_contact.turrets(), i);

                let armament_direction_target = Angle::from(mouse_position - transform.position);

                let mut angle_diff = (armament_direction_target - transform.direction).abs();
                if armament.vertical
                    || armament_entity_data.kind == EntityKind::Aircraft
                    || armament_entity_data.sub_kind == EntitySubKind::Depositor
                    || armament_entity_data.sub_kind == EntitySubKind::DepthCharge
                    || armament_entity_data.sub_kind == EntitySubKind::Mine
                {
                    // Vertically-launched armaments can fire in any horizontal direction.
                    // Aircraft can quickly assume any direction.
                    // Depositors, depth charges, and mines are not constrained by direction.
                    angle_diff = Angle::ZERO;
                }

                max_angle_diff += match armament_entity_data.sub_kind {
                    EntitySubKind::Shell => Angle::from_degrees(30.0),
                    EntitySubKind::Rocket => Angle::from_degrees(45.0),
                    EntitySubKind::RocketTorpedo => Angle::from_degrees(75.0),
                    EntitySubKind::Torpedo if armament_entity_data.sensors.sonar.range > 0.0 => {
                        Angle::from_degrees(150.0)
                    }
                    _ => Angle::from_degrees(90.0),
                };

                if !angle_limit || angle_diff < max_angle_diff {
                    let distance_squared = mouse_position.distance_squared(transform.position);
                    let score = (angle_diff.to_degrees().powi(2) + distance_squared).sqrt();
                    // Bias towards earlier firing solutions to avoid flickering left vs. right
                    // when steering straight.
                    if best_armament.map(|(_, s)| score + 2.5 < s).unwrap_or(true) {
                        best_armament = Some((i, score));
                    }
                }
            }
        }

        best_armament.map(|(idx, _)| idx)
    }

    /// This approximates the server-based automatic anti aircraft gunfire, in the form
    /// of tracer particles and audio (return value is appropriate volume).
    pub fn simulate_anti_aircraft(
        boat: &Contact,
        contacts: &HashMap<EntityId, InterpolatedContact>,
        core_state: &CoreState,
        player_position: Vec2,
        airborne_particles: &mut ParticleLayer,
    ) -> f32 {
        let mut volume = 0.0;

        let data = boat.data();
        let mut rng = thread_rng();
        // Anti-aircraft particles.
        for InterpolatedContact {
            view: aa_target, ..
        } in contacts.values()
        {
            if aa_target.entity_type().map(|t| t.data().kind) != Some(EntityKind::Aircraft) {
                // Not an aircraft.
                continue;
            }

            let distance_squared = boat
                .transform()
                .position
                .distance_squared(aa_target.transform().position);
            if distance_squared > data.anti_aircraft_range().powi(2) {
                // Out of range.
                continue;
            }

            if rng.gen::<f32>() > data.anti_aircraft {
                // Not powerful enough.
                continue;
            }

            if core_state.are_friendly(boat.player_id(), aa_target.player_id()) {
                // Don't shoot at friendly aircraft.
                continue;
            }

            let time_of_flight = Particle::LIFESPAN * 0.6;
            let mut prediction = *aa_target.transform();
            prediction.do_kinematics(time_of_flight);
            prediction.position += gen_radius(&mut rng, 10.0);

            // Use current position not prediction, because that looks weird.
            let aa_gun = boat
                .transform()
                .closest_point_on_keel_to(data.length * 0.8, aa_target.transform().position);

            let vector = prediction.position - aa_gun;
            let distance = vector.length();
            if distance < 5.0 {
                // Too close.
                continue;
            }
            let normalized = vector / distance;
            let offset = 5.0 + data.width * 0.4 + rng.gen::<f32>() * 10.0;
            for i in 0..3 {
                airborne_particles.add(Particle {
                    position: aa_gun + normalized * (offset + i as f32),
                    velocity: normalized * (distance.max(30.0) * (1.0 / time_of_flight))
                        + gen_radius(&mut rng, 1.0),
                    color: -1.0,
                    radius: 0.5,
                    smoothness: 0.25,
                });
            }

            volume += Self::volume_at(player_position.distance(aa_gun))
        }

        volume
    }
}

/// This is useful for avoiding firing the same weapon twice, which reduces fire rate in a high
/// latency environment.
#[derive(Debug)]
pub struct FireRateLimiter {
    counters: Vec<u8>,
    update_rate_limiter: RateLimiter,
}

impl FireRateLimiter {
    pub fn new() -> Self {
        Self {
            counters: Vec::with_capacity(32),
            update_rate_limiter: RateLimiter::new(0.1),
        }
    }

    pub fn is_ready(&self, armament_index: u8) -> bool {
        self.counters
            .get(armament_index as usize)
            .map(|&v| v == 0)
            .unwrap_or(true)
    }

    pub fn are_all_ready(&self) -> bool {
        self.counters.iter().all(|&v| v == 0)
    }

    pub fn fired(&mut self, armament_index: u8) {
        while armament_index as usize >= self.counters.len() {
            self.counters.push(0);
        }
        self.counters[armament_index as usize] = 3;
    }

    pub fn update(&mut self, elapsed_seconds: f32) {
        if self.update_rate_limiter.update_ready(elapsed_seconds) {
            for counter in &mut self.counters {
                *counter = counter.saturating_sub(((elapsed_seconds * 10.0) as u8).max(1));
            }
        }
    }
}
