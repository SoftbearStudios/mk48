// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use common::contact::{Contact, ContactTrait};
use glam::Vec2;

pub struct Mk48Camera {
    /// In meters.
    pub interpolated_zoom: f32,
    /// Camera on death.
    pub saved_camera: Option<(Vec2, f32)>,
    /// 1 = normal.
    pub zoom_input: f32,
}

impl Default for Mk48Camera {
    fn default() -> Self {
        Self {
            interpolated_zoom: Self::DEFAULT_ZOOM_INPUT * Self::MENU_VISUAL_RANGE,
            saved_camera: None,
            zoom_input: Self::DEFAULT_ZOOM_INPUT,
        }
    }
}

impl Mk48Camera {
    #[cfg(debug_assertions)]
    const MIN_ZOOM: f32 = 0.001; // Allow zooming in far in debug mode.
    #[cfg(not(debug_assertions))]
    const MIN_ZOOM: f32 = 0.216; // 0.6×sqrt(1÷.6)^−4 aka 4 full steps to min zoom
    const MAX_ZOOM: f32 = 1.0; // has to be 1.0 for full view
    const DEFAULT_ZOOM_INPUT: f32 = 0.6; // not changed
    const ZOOM_SPEED: f32 = 1.2909944; // sqrt(1÷.6) aka 2 full steps to max zoom
    const MENU_VISUAL_RANGE: f32 = 300.0;

    /// Gets the proper camera to display the game.
    pub fn camera(&self, player_contact: Option<&Contact>, aspect_ratio: f32) -> (Vec2, f32) {
        let camera = if let Some(player_contact) = player_contact {
            player_contact.transform().position
        } else {
            self.saved_camera
                .map(|camera| camera.0)
                .unwrap_or(Vec2::ZERO)
        };

        let effective_zoom = if aspect_ratio > 1.0 {
            self.interpolated_zoom * aspect_ratio
        } else {
            self.interpolated_zoom
        };

        (camera, effective_zoom)
    }

    /// Interpolates the zoom level closer as if delta_seconds elapsed.
    /// If the player's ship exists, it's camera info is cached, such that it may be returned
    /// even after that ship sinks.
    pub fn update(&mut self, player_contact: Option<&Contact>, delta_seconds: f32, snap: bool) {
        let zoom = if let Some(player_contact) = player_contact {
            let camera = player_contact.transform().position;
            let zoom = player_contact.entity_type().unwrap().data().camera_range();
            self.saved_camera = Some((camera, zoom));
            zoom
        } else if let Some(saved_camera) = self.saved_camera {
            saved_camera.1
        } else {
            Self::MENU_VISUAL_RANGE
        } * self.truncated_zoom_input();

        if snap {
            self.interpolated_zoom = zoom;
        } else {
            self.interpolated_zoom +=
                (zoom - self.interpolated_zoom) * (6.0 * delta_seconds).min(1.0);
        }
    }

    pub fn zoom(&mut self, delta: f32) {
        // Use multiplicative zoom instead of additive for more fluid feeling.
        let next_zoom_input = self.zoom_input * Self::ZOOM_SPEED.powf(delta);
        self.zoom_input = next_zoom_input.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
    }

    // Get exact zoom if you don't use pinch to zoom.
    fn truncated_zoom_input(&self) -> f32 {
        const P: f64 = 65536.0;
        ((self.zoom_input as f64 * P).floor() * (1.0 / P)) as f32
    }
}
