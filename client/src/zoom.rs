// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use common::contact::{Contact, ContactTrait};
use glam::Vec2;

impl Mk48Game {
    /// Gets the proper camera to display the game.
    pub(crate) fn camera(
        &self,
        player_contact: Option<&Contact>,
        aspect_ratio: f32,
    ) -> (Vec2, f32) {
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
    pub(crate) fn update_camera(&mut self, player_contact: Option<&Contact>, delta_seconds: f32) {
        let zoom = if let Some(player_contact) = player_contact {
            let camera = player_contact.transform().position;
            let zoom = player_contact.entity_type().unwrap().data().camera_range();
            self.saved_camera = Some((camera, zoom));
            zoom
        } else if let Some(saved_camera) = self.saved_camera {
            saved_camera.1
        } else {
            300.0
        } * self.zoom_input;

        self.interpolated_zoom += (zoom - self.interpolated_zoom) * (6.0 * delta_seconds).min(1.0);
    }

    pub(crate) fn zoom(&mut self, delta: &f32) {
        self.zoom_input = (self.zoom_input + delta * 0.14).clamp(1.0 / 6.0, 1.0);
    }
}
