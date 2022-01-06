// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::angle::Angle;
use crate::unused_velocity2d::Velocity;
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::ops::Add;

/// Transform stores a position, direction, and single-component velocity (along the direction).
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
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

    /// do_kinematics updates the position field of a transform based on the direction and velocity fields.
    pub fn do_kinematics(&mut self, delta_seconds: f32) {
        self.position += self.direction.to_vec() * self.velocity.to_mps() * delta_seconds;
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
