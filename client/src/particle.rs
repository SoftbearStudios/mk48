// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::Vec2;

/// Particle represents a single particle and contains information about how to update it.
pub struct Particle {
    pub position: Vec2,
    // Currently unused.
    pub altitude: f32,
    pub velocity: Vec2,
    pub created: f32,
}

impl Particle {
    /// update applies kinematics to the particle and returns whether it should be removed.
    pub fn update(&mut self, delta_seconds: f32) -> bool {
        self.position += self.velocity * delta_seconds;
        self.velocity *= 0.25f32.powf(delta_seconds);
        self.velocity.length_squared() < 0.05
    }
}
