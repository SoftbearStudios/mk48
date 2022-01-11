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
    pub start_time: f32,
}

impl Animation {
    /// How many frames are played per second.
    const FRAMES_PER_SECOND: f32 = 35.0;

    pub fn new(
        name: &'static str,
        position: Vec2,
        altitude: f32,
        scale: f32,
        start_time: f32,
    ) -> Self {
        Self {
            name,
            position,
            altitude,
            scale,
            start_time,
        }
    }

    /// Gets the frame (caller is responsible for knowing whether the animation is over).
    pub fn frame(&self, time_seconds: f32) -> usize {
        ((time_seconds - self.start_time) * Self::FRAMES_PER_SECOND) as usize
    }
}
