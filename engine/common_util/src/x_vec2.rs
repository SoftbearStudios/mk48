// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};

macro_rules! x_vec2 {
    ($name: ident, $lower_name: ident, $int: ty, $hash: ident) => {
        #[derive(
            Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize,
        )]
        pub struct $name {
            pub x: $int,
            pub y: $int,
        }

        impl $name {
            pub const ZERO: Self = Self::splat(0);
            pub const ONE: Self = Self::splat(1);
            pub const MIN: Self = Self::splat(<$int>::MIN);
            pub const MAX: Self = Self::splat(<$int>::MAX);

            pub const fn new(x: $int, y: $int) -> Self {
                Self { x, y }
            }

            pub const fn splat(xy: $int) -> Self {
                Self { x: xy, y: xy }
            }

            pub fn as_vec2(&self) -> Vec2 {
                (*self).into()
            }

            pub fn rounded(vec2: Vec2) -> Self {
                Self {
                    x: vec2.x.round() as $int,
                    y: vec2.y.round() as $int,
                }
            }

            pub fn floor(vec2: Vec2) -> Self {
                Self {
                    x: vec2.x.floor() as $int,
                    y: vec2.y.floor() as $int,
                }
            }

            pub fn ceil(vec2: Vec2) -> Self {
                Self {
                    x: vec2.x.ceil() as $int,
                    y: vec2.y.ceil() as $int,
                }
            }

            pub fn max_element(&self) -> $int {
                self.x.max(self.y)
            }

            pub fn min_element(&self) -> $int {
                self.x.min(self.y)
            }

            pub fn min_components(&self, other: Self) -> Self {
                Self {
                    x: self.x.min(other.x),
                    y: self.y.min(other.y),
                }
            }

            pub fn max_components(&self, other: Self) -> Self {
                Self {
                    x: self.x.max(other.x),
                    y: self.y.max(other.y),
                }
            }

            pub fn saturating_add(&self, rhs: Self) -> Self {
                Self {
                    x: self.x.saturating_add(rhs.x),
                    y: self.y.saturating_add(rhs.y),
                }
            }

            pub fn saturating_sub(&self, rhs: Self) -> Self {
                Self {
                    x: self.x.saturating_sub(rhs.x),
                    y: self.y.saturating_sub(rhs.y),
                }
            }
        }

        impl Add<Self> for $name {
            type Output = Self;

            fn add(self, rhs: $name) -> Self::Output {
                Self {
                    x: self.x + rhs.x,
                    y: self.y + rhs.y,
                }
            }
        }

        impl AddAssign<Self> for $name {
            fn add_assign(&mut self, rhs: $name) {
                self.x += rhs.x;
                self.y += rhs.y;
            }
        }

        impl Sub<Self> for $name {
            type Output = Self;

            fn sub(self, rhs: $name) -> Self::Output {
                Self {
                    x: self.x - rhs.x,
                    y: self.y - rhs.y,
                }
            }
        }

        impl Mul<$int> for $name {
            type Output = Self;

            fn mul(self, rhs: $int) -> Self::Output {
                Self {
                    x: self.x * rhs,
                    y: self.y * rhs,
                }
            }
        }

        impl SubAssign<Self> for $name {
            fn sub_assign(&mut self, rhs: $name) {
                self.x -= rhs.x;
                self.y -= rhs.y;
            }
        }

        impl From<$name> for Vec2 {
            fn from($lower_name: $name) -> Self {
                Self::new($lower_name.x as f32, $lower_name.y as f32)
            }
        }

        impl From<Vec2> for $name {
            fn from(vec2: Vec2) -> Self {
                Self::new(vec2.x as $int, vec2.y as $int)
            }
        }

        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                state.$hash(unsafe { std::mem::transmute(*self) });
            }
        }
    };
}

x_vec2!(U8Vec2, u8vec2, u8, write_u16);
x_vec2!(I8Vec2, i8vec2, i8, write_u16);

impl I8Vec2 {
    /// # Panics
    ///
    /// In debug mode, if the absolute value of either component cannot be represented.
    pub fn abs(&self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }
}

x_vec2!(U16Vec2, u16vec2, u16, write_u32);

impl From<U8Vec2> for U16Vec2 {
    fn from(u8vec2: U8Vec2) -> Self {
        Self {
            x: u8vec2.x as u16,
            y: u8vec2.y as u16,
        }
    }
}

x_vec2!(I16Vec2, i16vec2, i16, write_u32);

impl I16Vec2 {
    /// # Panics
    ///
    /// In debug mode, if the absolute value of either component cannot be represented.
    pub fn abs(&self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }
}

impl From<I8Vec2> for I16Vec2 {
    fn from(i8vec2: I8Vec2) -> Self {
        Self {
            x: i8vec2.x as i16,
            y: i8vec2.y as i16,
        }
    }
}
