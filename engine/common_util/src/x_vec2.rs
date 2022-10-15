// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{IVec2, UVec2, Vec2};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::hash::{Hash, Hasher};
use std::ops::*;

macro_rules! x_vec2_maybe_op {
    ($op: ident) => {
        #[inline]
        pub fn $op(&self, rhs: Self) -> Option<Self> {
            Some(Self {
                x: self.x.$op(rhs.x)?,
                y: self.y.$op(rhs.y)?,
            })
        }
    };
}

macro_rules! x_vec2_maybe_ops {
    ($($op:ident),+) => {
        $(x_vec2_maybe_op!($op);)+
    }
}

macro_rules! x_vec2_op {
    ($op: ident) => {
        #[inline]
        pub fn $op(&self, rhs: Self) -> Self {
            Self {
                x: self.x.$op(rhs.x),
                y: self.y.$op(rhs.y),
            }
        }
    };
}

macro_rules! x_vec2_ops {
    ($($op:ident),+) => {
        $(x_vec2_op!($op);)+
    }
}

macro_rules! x_vec2 {
    ($name: ident, $int: ty, $align: literal, $hash: ident) => {
        #[derive(
            Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize,
        )]
        #[repr(C, align($align))]
        pub struct $name {
            pub x: $int,
            pub y: $int,
        }

        impl $name {
            pub const ZERO: Self = Self::splat(0);
            pub const ONE: Self = Self::splat(1);
            pub const MIN: Self = Self::splat(<$int>::MIN);
            pub const MAX: Self = Self::splat(<$int>::MAX);

            #[inline]
            pub const fn new(x: $int, y: $int) -> Self {
                Self { x, y }
            }

            #[inline]
            pub const fn splat(xy: $int) -> Self {
                Self { x: xy, y: xy }
            }

            #[inline]
            pub fn as_vec2(&self) -> Vec2 {
                (*self).into()
            }

            #[inline]
            pub fn rounded(vec2: Vec2) -> Self {
                Self {
                    x: vec2.x.round() as $int,
                    y: vec2.y.round() as $int,
                }
            }

            #[inline]
            pub fn floor(vec2: Vec2) -> Self {
                Self {
                    x: vec2.x.floor() as $int,
                    y: vec2.y.floor() as $int,
                }
            }

            #[inline]
            pub fn ceil(vec2: Vec2) -> Self {
                Self {
                    x: vec2.x.ceil() as $int,
                    y: vec2.y.ceil() as $int,
                }
            }

            #[inline]
            pub fn max_element(&self) -> $int {
                self.x.max(self.y)
            }

            #[inline]
            pub fn min_element(&self) -> $int {
                self.x.min(self.y)
            }

            #[inline]
            pub fn min_components(&self, other: Self) -> Self {
                Self {
                    x: self.x.min(other.x),
                    y: self.y.min(other.y),
                }
            }

            #[inline]
            pub fn max_components(&self, other: Self) -> Self {
                Self {
                    x: self.x.max(other.x),
                    y: self.y.max(other.y),
                }
            }

            x_vec2_maybe_ops!(checked_add, checked_sub, checked_mul, checked_div);
            x_vec2_ops!(
                saturating_add,
                saturating_sub,
                saturating_mul,
                saturating_div
            );
            x_vec2_ops!(wrapping_add, wrapping_sub, wrapping_mul, wrapping_div);
        }

        impl Add<Self> for $name {
            type Output = Self;

            #[inline]
            fn add(self, rhs: $name) -> Self::Output {
                Self {
                    x: self.x + rhs.x,
                    y: self.y + rhs.y,
                }
            }
        }

        impl AddAssign<Self> for $name {
            #[inline]
            fn add_assign(&mut self, rhs: $name) {
                *self = *self + rhs;
            }
        }

        impl Sub<Self> for $name {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: $name) -> Self::Output {
                Self {
                    x: self.x - rhs.x,
                    y: self.y - rhs.y,
                }
            }
        }

        impl SubAssign<Self> for $name {
            #[inline]
            fn sub_assign(&mut self, rhs: $name) {
                *self = *self - rhs;
            }
        }

        impl Add<$int> for $name {
            type Output = Self;

            #[inline]
            fn add(self, rhs: $int) -> Self::Output {
                Self {
                    x: self.x + rhs,
                    y: self.y + rhs,
                }
            }
        }

        impl AddAssign<$int> for $name {
            #[inline]
            fn add_assign(&mut self, rhs: $int) {
                *self = *self + rhs;
            }
        }

        impl Sub<$int> for $name {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: $int) -> Self::Output {
                Self {
                    x: self.x - rhs,
                    y: self.y - rhs,
                }
            }
        }

        impl SubAssign<$int> for $name {
            #[inline]
            fn sub_assign(&mut self, rhs: $int) {
                *self = *self - rhs;
            }
        }

        impl Mul<$int> for $name {
            type Output = Self;

            #[inline]
            fn mul(self, rhs: $int) -> Self::Output {
                Self {
                    x: self.x * rhs,
                    y: self.y * rhs,
                }
            }
        }

        impl MulAssign<$int> for $name {
            #[inline]
            fn mul_assign(&mut self, rhs: $int) {
                *self = *self * rhs;
            }
        }

        impl Div<$int> for $name {
            type Output = Self;

            #[inline]
            fn div(self, rhs: $int) -> Self::Output {
                Self {
                    x: self.x / rhs,
                    y: self.y / rhs,
                }
            }
        }

        impl DivAssign<$int> for $name {
            #[inline]
            fn div_assign(&mut self, rhs: $int) {
                *self = *self / rhs;
            }
        }

        impl From<$name> for Vec2 {
            #[inline]
            fn from(v: $name) -> Self {
                Self::new(v.x as f32, v.y as f32)
            }
        }

        impl From<Vec2> for $name {
            #[inline]
            fn from(vec2: Vec2) -> Self {
                Self::new(vec2.x as $int, vec2.y as $int)
            }
        }

        impl Hash for $name {
            #[inline]
            fn hash<H: Hasher>(&self, state: &mut H) {
                state.$hash(unsafe { std::mem::transmute(*self) });
            }
        }
    };
}

x_vec2!(U8Vec2, u8, 2, write_u16);
x_vec2!(I8Vec2, i8, 2, write_u16);
x_vec2!(U16Vec2, u16, 4, write_u32);
x_vec2!(I16Vec2, i16, 4, write_u32);

impl I8Vec2 {
    /// # Panics
    ///
    /// In debug mode, if the absolute value of either component cannot be represented.
    #[inline]
    pub fn abs(&self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }
}

impl From<U8Vec2> for U16Vec2 {
    #[inline]
    fn from(u8vec2: U8Vec2) -> Self {
        Self {
            x: u8vec2.x as u16,
            y: u8vec2.y as u16,
        }
    }
}

// TODO replace with From impl.
impl Into<UVec2> for U16Vec2 {
    #[inline]
    fn into(self) -> UVec2 {
        UVec2::new(self.x as u32, self.y as u32)
    }
}

impl TryFrom<UVec2> for U16Vec2 {
    type Error = <u32 as TryInto<u16>>::Error;

    fn try_from(v: UVec2) -> Result<Self, Self::Error> {
        Ok(Self::new(v.x.try_into()?, v.y.try_into()?))
    }
}

impl U16Vec2 {
    #[inline]
    pub fn as_uvec2(self) -> UVec2 {
        self.into()
    }
}

impl I16Vec2 {
    /// # Panics
    ///
    /// In debug mode, if the absolute value of either component cannot be represented.
    #[inline]
    pub fn abs(&self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }
}

impl From<I8Vec2> for I16Vec2 {
    #[inline]
    fn from(i8vec2: I8Vec2) -> Self {
        Self {
            x: i8vec2.x as i16,
            y: i8vec2.y as i16,
        }
    }
}

// TODO replace with From impl.
impl Into<IVec2> for I16Vec2 {
    #[inline]
    fn into(self) -> IVec2 {
        IVec2::new(self.x as i32, self.y as i32)
    }
}

impl TryFrom<IVec2> for I16Vec2 {
    type Error = <i32 as TryInto<i16>>::Error;

    fn try_from(v: IVec2) -> Result<Self, Self::Error> {
        Ok(Self::new(v.x.try_into()?, v.y.try_into()?))
    }
}

impl I16Vec2 {
    #[inline]
    pub fn as_ivec2(self) -> IVec2 {
        self.into()
    }
}
