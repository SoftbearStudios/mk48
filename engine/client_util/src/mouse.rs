// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apply::Apply;
use glam::Vec2;
use variant_count::VariantCount;

/// Identifies a mouse button (left, middle, or right).
#[derive(Copy, Clone, Eq, PartialEq, VariantCount)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

impl MouseButton {
    /// Converts from JS mouse button, if possible.
    pub fn try_from_button(mouse_button: i16) -> Option<Self> {
        Some(match mouse_button {
            0 => Self::Left,
            1 => Self::Middle,
            2 => Self::Right,
            _ => return None,
        })
    }
}

/// The state of one mouse button.
#[derive(Default, Copy, Clone)]
pub enum MouseButtonState {
    /// The button was pressed and released fast enough to form a click.
    /// This state will persist until manually cleared (see `MouseState::take_click`)
    Click,
    /// Stores the time the button was pressed.
    Down(f32),
    #[default]
    Up,
}

impl MouseButtonState {
    /// If the mouse is released within this time, it is considered a click.
    pub const MAX_CLICK_TIME: f32 = 0.180;

    /// Whether the mouse is up (after a click).
    pub fn is_click(&self) -> bool {
        matches!(self, Self::Click)
    }

    /// Whether the mouse is up (after a click). Resets state back to up (no click).
    pub fn take_click(&mut self) -> bool {
        if self.is_click() {
            *self = Self::Up;
            true
        } else {
            false
        }
    }

    /// Whether the mouse is down (or clicking).
    pub fn is_down(&self) -> bool {
        matches!(self, Self::Down(_))
    }

    /// Whether the mouse is down (for too long to be clicking).
    pub fn is_down_not_click(&self, time: f32) -> bool {
        if let &Self::Down(t) = self {
            time > t + Self::MAX_CLICK_TIME
        } else {
            false
        }
    }

    /// Whether the mouse is up (no past click).
    pub fn is_up(&self) -> bool {
        matches!(self, Self::Up)
    }
}

/// Any type of mouse event. `Self::Wheel` may be emulated by any zooming intent.
pub enum MouseEvent {
    Button {
        button: MouseButton,
        down: bool,
        time: f32,
    },
    Wheel(f32),
    /// Position in view space (-1..1).
    Move(Vec2),
}

/// The state of the mouse i.e. buttons and position.
#[derive(Default)]
pub struct MouseState {
    states: [MouseButtonState; MouseButton::VARIANT_COUNT],
    /// Position in view space (-1..1).
    pub position: Vec2,
    /// During a pinch to zoom gesture, stores last distance value.
    pub(crate) pinch_distance: Option<f32>,
}

impl Apply<MouseEvent> for MouseState {
    fn apply(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::Button { button, down, time } => {
                if down {
                    if !self.state(button).is_down() {
                        *self.state_mut(button) = MouseButtonState::Down(time);
                    }
                } else if let MouseButtonState::Down(t) = self.state(button) {
                    *self.state_mut(button) = if time <= t + MouseButtonState::MAX_CLICK_TIME {
                        MouseButtonState::Click
                    } else {
                        MouseButtonState::Up
                    }
                } else {
                    *self.state_mut(button) = MouseButtonState::Up;
                }
            }
            MouseEvent::Move(position) => {
                self.position = position;
            }
            _ => {}
        }
    }
}

impl MouseState {
    /// Immutable reference to the state of a particular button.
    pub fn state(&self, button: MouseButton) -> &MouseButtonState {
        &self.states[button as usize]
    }

    /// Mutable reference to the state of a particular button.
    pub(crate) fn state_mut(&mut self, button: MouseButton) -> &mut MouseButtonState {
        &mut self.states[button as usize]
    }

    /// See `MouseButtonState::is_click`.
    pub fn is_click(&self, button: MouseButton) -> bool {
        self.state(button).is_click()
    }

    /// See `MouseButtonState::take_click`.
    pub fn take_click(&mut self, button: MouseButton) -> bool {
        self.state_mut(button).take_click()
    }

    /// See `MouseButtonState::is_down`.
    pub fn is_down(&self, button: MouseButton) -> bool {
        self.state(button).is_down()
    }

    /// See `MouseButtonState::is_down_not_click`.
    pub fn is_down_not_click(&self, button: MouseButton, time: f32) -> bool {
        self.state(button).is_down_not_click(time)
    }

    /// See `MouseButtonState::is_up`.
    pub fn is_up(&self, button: MouseButton) -> bool {
        self.state(button).is_up()
    }
}
