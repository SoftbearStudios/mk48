// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ticks::{Ticks, TicksRepr};
use core_protocol::serde_util::{F32Visitor, I16Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

type VelocityRepr = i16;

// Note: pub(crate) is intentional.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Velocity(pub VelocityRepr);

/// Velocity efficiently stores a signed speed.
#[allow(dead_code)]
impl Velocity {
    /// Zero velocity (at rest).
    pub const ZERO: Self = Self(0);
    /// Smallest representable positive velocity.
    pub const UNIT: Self = Self(1);
    /// Minimum (negative) velocity.
    pub const MIN: Self = Self(VelocityRepr::MIN);
    /// Maximum possible velocity.
    pub const MAX: Self = Self(VelocityRepr::MAX);
    /// Inverse of scale.
    const INV_SCALE: u32 = 1 << 5;
    /// How many meters per second per unit of velocity.
    const SCALE: f32 = 1.0 / Self::INV_SCALE as f32;
    /// How many knots per unit of velocity.
    const KNOTS_SCALE: f32 = Self::SCALE * 1.94384;
    /// Max reverse velocity as a function of max forward velocity.
    pub const MAX_REVERSE_SCALE: f32 = -1.0 / 3.0;

    /// new returns zero Velocity.
    pub fn new() -> Self {
        Self::ZERO
    }

    /// to_mps returns an amount of meters per second corresponding to the Velocity.
    #[inline]
    pub fn to_mps(self) -> f32 {
        self.0 as f32 * Self::SCALE
    }

    /// from_mps returns a Velocity from a given amount of meters per second.
    #[inline]
    pub fn from_mps(mps: f32) -> Self {
        Self((mps * (1.0 / Self::SCALE)) as VelocityRepr)
    }

    /// from_mps returns a Velocity from a given amount of centimeters per second.
    pub const fn from_whole_cmps(cmps: u32) -> Self {
        let scaled = cmps * Self::INV_SCALE / 100;
        if scaled > VelocityRepr::MAX as u32 {
            debug_assert!(false, "from_whole_cmps overflow");
            Self::MAX
        } else {
            Self(scaled as VelocityRepr)
        }
    }

    /// to_knots returns an amount of knots corresponding to the Velocity.
    #[inline]
    pub fn to_knots(self) -> f32 {
        self.0 as f32 * Self::KNOTS_SCALE
    }

    /// from_knots returns a velocity from a given amount of knots.
    #[inline]
    pub fn from_knots(knots: f32) -> Self {
        Self((knots * (1.0 / Self::KNOTS_SCALE)) as VelocityRepr)
    }

    /// clamp returns the velocity, clamped between min and max.
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self(self.0.clamp(min.0, max.0) as VelocityRepr)
    }

    /// clamp_magnitude returns the original Velocity such that its magnitude is less than or
    /// equal to max (which must be non-negative).
    pub fn clamp_magnitude(self, max: Self) -> Self {
        debug_assert!(max.0 >= 0);
        self.clamp(-max, max)
    }

    /// abs returns the absolute value of a Velocity.
    pub fn abs(self) -> Self {
        Self(self.0.abs() as VelocityRepr)
    }

    /// difference returns the positive difference between two velocities.
    pub fn difference(self, other: Self) -> Self {
        if self < other {
            other - self
        } else {
            self - other
        }
    }

    /// lerp linearly interpolates between velocities.
    pub fn lerp(self, other: Self, value: f32) -> Self {
        self + (other - self) * value
    }
}

impl Default for Velocity {
    /// default returns zero Velocity.
    fn default() -> Self {
        Self::ZERO
    }
}

impl Add for Velocity {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.saturating_add(other.0))
    }
}

impl AddAssign for Velocity {
    fn add_assign(&mut self, other: Self) {
        self.0 = self.0.saturating_add(other.0);
    }
}

impl Sub for Velocity {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.saturating_sub(other.0))
    }
}

impl SubAssign for Velocity {
    fn sub_assign(&mut self, other: Self) {
        self.0 = self.0.saturating_sub(other.0);
    }
}

impl Neg for Velocity {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::ZERO - self
    }
}

impl Mul<f32> for Velocity {
    type Output = Self;

    fn mul(self, other: f32) -> Self::Output {
        Self((self.0 as f32 * other) as VelocityRepr)
    }
}

impl Mul<Ticks> for Velocity {
    type Output = Self;

    fn mul(self, other: Ticks) -> Self::Output {
        debug_assert!(other.0 < VelocityRepr::MAX as TicksRepr);
        Self((self.0.saturating_mul(other.0 as VelocityRepr)) as VelocityRepr)
    }
}

impl fmt::Debug for Velocity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_mps())
    }
}

impl Serialize for Velocity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_f32(self.to_mps())
        } else {
            serializer.serialize_i16(self.0)
        }
    }
}

impl<'de> Deserialize<'de> for Velocity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_f32(F32Visitor).map(Self::from_mps)
        } else {
            deserializer.deserialize_i16(I16Visitor).map(Self)
        }
    }
}

#[cfg(test)]
mod tests {
    // use crate::velocity::Velocity;

    // TODO: Test velocity.
}
