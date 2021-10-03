// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::MouseButton;
use common::altitude::Altitude;
use common::entity::*;
use glam::Vec2;
use instant::Duration;
use instant::Instant;

/// Input keeps track of the simplest types of user input.
#[derive(Debug)]
pub struct Input {
    pub mouse_left_click: bool,
    mouse_left_down_time: Option<Instant>,
    pub mouse_right_down: bool,
    pub mouse_position: Vec2,
    pub shoot: bool,
    pub pay: bool,
    pub stop: bool,
    pub joystick: Option<Vec2>, // Some if joystick is active, None otherwise.
    pub active: bool,
    pub altitude_target: Altitude,
    pub armament_selection: Option<(EntityKind, EntitySubKind)>,
    zoom: f32,
}

impl Input {
    /// Mouse clicks that last MOUSE_CLICK_MAX or less are considered clicks. Otherwise, they are considered
    /// clicking and holding.
    const MOUSE_CLICK_MAX: Duration = Duration::from_millis(180); // Seconds

    pub fn new() -> Self {
        Self {
            mouse_left_click: false,
            mouse_left_down_time: None,
            mouse_right_down: false,
            mouse_position: Vec2::ZERO,
            shoot: false,
            pay: false,
            stop: false,
            joystick: None,
            active: true,
            altitude_target: Altitude::ZERO,
            armament_selection: None,
            zoom: 0.5,
        }
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    // Returns true if and only if the left mouse button is down long enough for it to not represent
    // a click if it was to be released.
    pub fn mouse_left_down_not_click(&self) -> bool {
        self.mouse_left_down_time
            .map(|time| time.elapsed() > Self::MOUSE_CLICK_MAX)
            .unwrap_or(false)
    }

    // Resets input flags.
    pub fn reset(&mut self) {
        self.mouse_left_click = false;
    }

    pub fn handle_mouse_button(&mut self, button: MouseButton, down: bool) {
        match button {
            MouseButton::Left => {
                if down {
                    self.mouse_left_down_time = Some(Instant::now());
                } else if let Some(mouse_left_down_time) = self.mouse_left_down_time {
                    if mouse_left_down_time.elapsed() <= Self::MOUSE_CLICK_MAX {
                        self.mouse_left_click = true;
                    }
                    self.mouse_left_down_time = None;
                }
            }
            MouseButton::Right => self.mouse_right_down = down,
        }
    }

    pub fn handle_mouse_move(&mut self, position: Vec2) {
        self.mouse_position = position;
    }

    pub fn handle_wheel(&mut self, delta: f32) {
        self.zoom = (self.zoom + delta * 0.04).clamp(1.0 / 6.0, 1.0);
    }

    pub fn handle_joystick(&mut self, position: Option<Vec2>, stop: bool) {
        self.joystick = position;
        self.stop = stop;
    }
}
