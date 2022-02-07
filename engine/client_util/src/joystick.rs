// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::keyboard::{Key, KeyState, KeyboardState};
use common_util::range::map_ranges;
use glam::Vec2;

/// Helper for taking joystick input. For now, the only joystick input is emulated from keyboard input.
#[derive(Debug)]
pub struct Joystick {
    /// Turning (x axis) is interpolated.
    pub position: Vec2,
    /// X and Y are treated same.
    pub translation_2d: Vec2,
    pub stop: bool,
}

impl Joystick {
    /// Returns Some if the keyboard state can reasonably interpreted as a joystick-style input.
    ///  - WASD/Arrow keys for movement.
    ///  - X for stop.
    pub fn try_from_keyboard_state(
        time_seconds: f32,
        keyboard_state: &KeyboardState,
    ) -> Option<Self> {
        let mut forward_backward = 0f32;
        let mut left_right = 0f32;
        let mut left_right_translation = 0f32;
        if keyboard_state
            .state(Key::W)
            .combined(keyboard_state.state(Key::Up))
            .is_down()
        {
            forward_backward += 1.0;
        }
        if keyboard_state
            .state(Key::S)
            .combined(keyboard_state.state(Key::Down))
            .is_down()
        {
            forward_backward -= 1.0;
        }
        if let KeyState::Down(start) = keyboard_state
            .state(Key::A)
            .combined(keyboard_state.state(Key::Left))
        {
            let elapsed = time_seconds - start;
            left_right += map_ranges(elapsed, 0.0..1.0, 0.25..1.0, true);
            left_right_translation -= 1.0;
        }
        if let KeyState::Down(start) = keyboard_state
            .state(Key::D)
            .combined(keyboard_state.state(Key::Right))
        {
            let elapsed = time_seconds - start;
            left_right -= map_ranges(elapsed, 0.0..1.0, 0.25..1.0, true);
            left_right_translation += 1.0;
        }
        let stop = keyboard_state.is_down(Key::X);

        if !stop && forward_backward == 0.0 && left_right == 0.0 {
            None
        } else {
            Some(Self {
                position: Vec2::new(left_right, forward_backward),
                translation_2d: Vec2::new(left_right_translation, forward_backward),
                stop,
            })
        }
    }
}
