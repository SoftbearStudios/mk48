// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::Vec2;

/// Tracks the state of an animation e.g. explosion.
pub struct Animation {
    /// One of animations defined by the sprite sheet.
    pub name: &'static str,
    pub position: Vec2,
    pub altitude: f32,
    pub scale: f32,
    pub frame: usize,
}

impl Animation {
    pub fn new(name: &'static str, position: Vec2, altitude: f32, scale: f32) -> Self {
        Self {
            name,
            position,
            altitude,
            scale,
            frame: 0,
        }
    }

    /// Increments the animation frame if necessary.
    pub fn update(&mut self, delta_seconds: f32) {
        self.frame += (delta_seconds * (1.0 / 20.0)).max(1.0) as usize;
    }
}
