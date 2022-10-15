// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apply::Apply;
use std::num::NonZeroU8;
use strum_macros::Display;

/// Each variant is a possible key. Not guaranteed to support all keys.
#[derive(Copy, Clone, Eq, PartialEq, Display)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,

    Alt,
    Backspace,
    Ctrl,
    Down,
    Enter,
    Home,
    Left,
    PageDown,
    PageUp,
    MinusUnderscore,
    EqualsPlus,
    Right,
    Shift,
    Space,
    Tab,
    Up,
}

impl Key {
    /// Converts from a Javascript keycode.
    pub fn try_from_key_code(key_code: u32) -> Option<Self> {
        Some(match key_code {
            8 => Self::Backspace,
            9 => Self::Tab,
            13 => Self::Enter,
            16 => Self::Shift,
            17 => Self::Ctrl,
            18 => Self::Alt,
            32 => Self::Space,
            33 => Self::PageUp,
            34 => Self::PageDown,
            37 => Self::Left,
            38 => Self::Up,
            39 => Self::Right,
            40 => Self::Down,
            48 => Self::Zero,
            49 => Self::One,
            50 => Self::Two,
            51 => Self::Three,
            52 => Self::Four,
            53 => Self::Five,
            54 => Self::Six,
            55 => Self::Seven,
            56 => Self::Eight,
            57 => Self::Nine,
            65 => Self::A,
            66 => Self::B,
            67 => Self::C,
            68 => Self::D,
            69 => Self::E,
            70 => Self::F,
            71 => Self::G,
            72 => Self::H,
            73 => Self::I,
            74 => Self::J,
            75 => Self::K,
            76 => Self::L,
            77 => Self::M,
            78 => Self::N,
            79 => Self::O,
            80 => Self::P,
            81 => Self::Q,
            82 => Self::R,
            83 => Self::S,
            84 => Self::T,
            85 => Self::U,
            86 => Self::V,
            87 => Self::W,
            88 => Self::X,
            89 => Self::Y,
            90 => Self::Z,
            187 => Self::EqualsPlus,
            189 => Self::MinusUnderscore,
            _ => return None,
        })
    }

    pub fn digit(self) -> Option<u8> {
        Some(match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
            Self::Five => 5,
            Self::Six => 6,
            Self::Seven => 7,
            Self::Eight => 8,
            Self::Nine => 9,
            _ => return None,
        })
    }

    /// Digit but 0 is mapped to ten.
    pub fn digit_with_ten(self) -> Option<NonZeroU8> {
        self.digit()
            .map(|d| NonZeroU8::new(d).unwrap_or(NonZeroU8::new(10).unwrap()))
    }
}

/// The state of any key.
#[derive(Default, Copy, Clone)]
pub enum KeyState {
    /// Stores when, in game time, the key was pressed.
    Down(f32),
    #[default]
    Up,
}

impl KeyState {
    /// Is the key down.
    pub fn is_down(&self) -> bool {
        matches!(self, Self::Down(_))
    }

    /// Is the key up.
    pub fn is_up(&self) -> bool {
        matches!(self, Self::Up)
    }

    /// If both keys are down, combined key state is down (time is earlier of the two).
    /// If only one key is down, combined key state is down with same time.
    /// If both keys are up, combined key state is up.
    pub fn combined(&self, other: &Self) -> Self {
        Self::Down(match (self, other) {
            (Self::Down(t1), &Self::Down(t2)) => t1.min(t2),
            (&Self::Down(t), Self::Up) => t,
            (Self::Up, &Self::Down(t)) => t,
            (Self::Up, Self::Up) => return Self::Up,
        })
    }
}

/// The entire current state of the keyboard.
pub struct KeyboardState {
    pub(crate) states: [KeyState; std::mem::variant_count::<Key>()],
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self {
            states: [KeyState::Up; std::mem::variant_count::<Key>()],
        }
    }
}

impl Apply<KeyboardEvent> for KeyboardState {
    fn apply(&mut self, event: KeyboardEvent) {
        if self.state(event.key).is_down() != event.down {
            *self.state_mut(event.key) = event
                .down
                .then(|| KeyState::Down(event.time))
                .unwrap_or(KeyState::Up);
        }
    }
}

impl KeyboardState {
    /// Immutable reference to state of one key.
    pub fn state(&self, key: Key) -> &KeyState {
        &self.states[key as usize]
    }

    /// Mutable reference to state of one key.
    pub(crate) fn state_mut(&mut self, key: Key) -> &mut KeyState {
        &mut self.states[key as usize]
    }

    /// See `KeyState::is_down`.
    pub fn is_down(&self, key: Key) -> bool {
        self.state(key).is_down()
    }

    /// See `KeyState::is_up`.
    pub fn is_up(&self, key: Key) -> bool {
        self.state(key).is_up()
    }
}

/// A key up or key down event, containing some additional context.
pub struct KeyboardEvent {
    pub key: Key,
    pub ctrl: bool,
    pub down: bool,
    pub shift: bool,
    pub time: f32,
}
