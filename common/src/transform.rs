// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::angle::Angle;
use crate::entity::{EntityData, EntityKind, EntitySubKind};
use crate::guidance::Guidance;
use crate::velocity::Velocity;
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::ops::Add;

/// Transform stores a position, direction, and single-component velocity (along the direction).
#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec2,
    pub direction: Angle,
    pub velocity: Velocity,
}

/// A transform with added dimensions (aligned to the axis defined by transform.direction).
#[derive(Copy, Clone, Debug, Default)]
pub struct DimensionTransform {
    pub transform: Transform,
    pub dimensions: Vec2,
}

impl Transform {
    /// new returns a zero Transform.
    pub fn new() -> Self {
        Self::default()
    }

    /// from_position returns a Transform with a position and zero angle/velocity.
    pub fn from_position(position: Vec2) -> Self {
        Self {
            position,
            ..Self::new()
        }
    }

    /// apply_guidance modifies a Transform according to a Guidance.
    pub fn apply_guidance(
        &mut self,
        data: &EntityData,
        guidance: Guidance,
        mut max_speed: f32,
        delta_seconds: f32,
    ) {
        debug_assert!(max_speed >= 0.0);
        debug_assert!(delta_seconds >= 0.0);
        max_speed = max_speed.min(data.speed.to_mps());

        // Collectibles don't turn with guidance.
        // Shells and rockets (at least the ones currently in the game) can't turn.
        // Mines and depth charges have no control surfaces.
        if data.kind != EntityKind::Collectible
            && !matches!(
                data.sub_kind,
                EntitySubKind::Shell
                    | EntitySubKind::Rocket
                    | EntitySubKind::Mine
                    | EntitySubKind::DepthCharge
            )
        {
            let delta_angle = guidance.direction_target - self.direction;
            let turn_max = Angle::from_radians(
                (delta_seconds
                    * match data.kind {
                        // Longer boats turn slower.
                        EntityKind::Boat => 0.125 + 20.0 / data.length,
                        // Everything else turns slower if moving faster.
                        EntityKind::Aircraft => {
                            2.0 * (1.0 - self.velocity.abs().to_mps() / (1.0 + data.speed.to_mps()))
                                .max(0.5)
                        }
                        _ => (1.0 - self.velocity.abs().to_mps() / (1.0 + data.speed.to_mps()))
                            .max(0.3),
                    })
                .clamp(0.0, std::f32::consts::PI),
            );
            self.direction += delta_angle.clamp_magnitude(turn_max);

            // Allow torpedoes to make a u-turn without getting too far off track.
            // Never will activate with only automatic homing.
            if data.sub_kind == EntitySubKind::Torpedo
                && delta_angle.abs() > Angle::from_degrees(80.0)
            {
                max_speed *= 1.0 / 3.0
            }
        }

        // Velocity is brought within acceptable parameters not by clamping it directly,
        // but by always moving towards an in-range target velocity (clamped here). This is
        // so that zero-max-speed objects like collectibles can temporarily experience
        // non-zero velocity.
        let delta_velocity = guidance
            .velocity_target
            .to_mps()
            .clamp(max_speed * Velocity::MAX_REVERSE_SCALE, max_speed)
            - self.velocity.to_mps();

        // Delta cannot be proportional to possibly-zero max_speed, because zero-max-speed
        // depth charges must be able to dissipate initial velocity, so clamp minimum.
        // Clamp maximum acceleration to limit faster missiles.
        let max_accel = 1.0 / 3.0 * delta_seconds * max_speed.clamp(15.0, 500.0);
        self.velocity = Velocity::from_mps(
            self.velocity.to_mps()
                + delta_velocity.clamp(
                    if data.sub_kind == EntitySubKind::Heli {
                        -5.0 * max_accel
                    } else {
                        -max_accel
                    },
                    max_accel,
                ),
        );
    }

    /// do_kinematics updates the position field of a transform based on the direction and velocity fields.
    pub fn do_kinematics(&mut self, delta_seconds: f32) {
        self.position += self.direction.to_vec() * self.velocity.to_mps() * delta_seconds;
    }

    /// Closest point on self's keel (a line segment from bow to stern) to position.
    /// Tolerance is what fraction of the length of the keep to consider.
    pub fn closest_point_on_keel_to(&self, keel_length: f32, position: Vec2) -> Vec2 {
        let pos_diff = position - self.position;
        if pos_diff.length_squared() < 1.0 {
            self.position
        } else {
            self.position
                + pos_diff
                    .project_onto(self.direction.to_vec())
                    .clamp_length_max(keel_length * 0.5)
        }
    }
}

impl Add for Transform {
    type Output = Self;

    /// add composes two transforms together (e.g. `let weapon_transform = entity_transform.add(weapon_transform_relative_to_entity);`).
    fn add(mut self, rhs: Self) -> Self::Output {
        let normal: Vec2 = self.direction.into();
        self.position.x += rhs.position.x * normal.x - rhs.position.y * normal.y;
        self.position.y += rhs.position.x * normal.y + rhs.position.y * normal.x;
        self.direction += rhs.direction;
        let new_normal: Vec2 = self.direction.into();
        self.velocity = self.velocity * normal.dot(new_normal) + rhs.velocity;
        self
    }
}
