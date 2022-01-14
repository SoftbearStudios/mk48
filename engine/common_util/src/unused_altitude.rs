// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::range::map_ranges;
use crate::ticks::{Ticks, TicksRepr};
use core_protocol::serde_util::{F32Visitor, I8Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

type UnscaledAltitude = i8;

// Note: pub(crate) is intentional.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Altitude<const SCALE: usize>(pub UnscaledAltitude);

/// Scaled altitude
#[allow(dead_code)]
impl<const SCALE: usize> Altitude<SCALE> {
    pub const ZERO: Self = Self(0);
    pub const UNIT: Self = Self(1);
    pub const MIN: Self = Self(UnscaledAltitude::MIN);
    pub const MAX: Self = Self(UnscaledAltitude::MAX);

    pub fn new() -> Self {
        Self::ZERO
    }

    pub fn to_meters(self) -> f32 {
        self.0 as f32 * SCALE as f32
    }

    pub fn from_meters(meters: f32) -> Self {
        Self((meters * (1.0 / SCALE as f32)) as UnscaledAltitude)
    }

    // The u8 is interpreted as 0-255 meaning MIN-MAX.
    pub const fn from_u8(alt: u8) -> Self {
        Self((alt as i16 - 128) as UnscaledAltitude)
    }

    // The u8 is output as MIN-MAX meaning 0-255.
    pub fn to_u8(self) -> u8 {
        (self.0 as i16 + 128) as u8
    }

    pub fn from_norm(norm: f32) -> Self {
        Self(map_ranges(
            norm,
            -1.0..1.0,
            Self::MIN.0 as f32..Self::MAX.0 as f32,
            true,
        ) as UnscaledAltitude)
    }

    /// to_norm returns self in the range [-1, 1] where 0.0 is the midpoint.
    pub fn to_norm(self) -> f32 {
        map_ranges(
            self.0 as f32,
            Self::MIN.0 as f32..Self::MAX.0 as f32,
            -1.0..1.0,
            true,
        )
    }

    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0) as UnscaledAltitude)
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0) as UnscaledAltitude)
    }

    /// Returns self, clamped between min and max (max >= min or will panic).
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self(self.0.clamp(min.0, max.0) as UnscaledAltitude)
    }

    /// Argument must be positive.
    pub fn clamp_magnitude(self, max: Self) -> Self {
        debug_assert!(max.0 >= 0);
        self.clamp(-max, max)
    }

    /// Linearly interpolates towards another altitudes.
    pub fn lerp(self, end: Self, t: f32) -> Self {
        let diff = end - self;
        self + match end.cmp(&self) {
            Ordering::Less => (diff * t).min(-Altitude::UNIT),
            Ordering::Greater => (diff * t).max(Altitude::UNIT),
            Ordering::Equal => Altitude::ZERO,
        }
    }

    /// Returns true if below sealevel (zero).
    pub fn is_below_sealevel(self) -> bool {
        self < Self::ZERO
    }

    /// Returns true if above sealevel (zero).
    pub fn is_above_sealevel(self) -> bool {
        self > Self::ZERO
    }

    /// Returns positive difference between two altitudes.
    pub fn difference(self, other: Self) -> Self {
        if self < other {
            other - self
        } else {
            self - other
        }
    }
}

impl<const SCALE: usize> Default for Altitude<SCALE> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<const SCALE: usize> Add for Altitude<SCALE> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.saturating_add(other.0))
    }
}

impl<const SCALE: usize> AddAssign for Altitude<SCALE> {
    fn add_assign(&mut self, other: Self) {
        self.0 = self.0.saturating_add(other.0);
    }
}

impl<const SCALE: usize> Sub for Altitude<SCALE> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.saturating_sub(other.0))
    }
}

impl<const SCALE: usize> SubAssign for Altitude<SCALE> {
    fn sub_assign(&mut self, other: Self) {
        self.0 = self.0.saturating_sub(other.0);
    }
}

impl<const SCALE: usize> Neg for Altitude<SCALE> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::ZERO - self
    }
}

impl<const SCALE: usize> Mul<f32> for Altitude<SCALE> {
    type Output = Self;

    fn mul(self, other: f32) -> Self::Output {
        Self((self.0 as f32 * other) as UnscaledAltitude)
    }
}

impl<const SCALE: usize> Mul<Ticks> for Altitude<SCALE> {
    type Output = Self;

    fn mul(self, other: Ticks) -> Self::Output {
        debug_assert!(other.0 < UnscaledAltitude::MAX as TicksRepr);
        Self((self.0.saturating_mul(other.0 as UnscaledAltitude)) as UnscaledAltitude)
    }
}

impl<const SCALE: usize> fmt::Debug for Altitude<SCALE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_meters())
    }
}

impl<const SCALE: usize> Serialize for Altitude<SCALE> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_f32(self.to_meters())
        } else {
            serializer.serialize_i8(self.0)
        }
    }
}

impl<'de, const SCALE: usize> Deserialize<'de> for Altitude<SCALE> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer
                .deserialize_f32(F32Visitor)
                .map(Self::from_meters)
        } else {
            deserializer.deserialize_i8(I8Visitor).map(Self)
        }
    }
}

#[cfg(test)]
mod tests {
    // use crate::altitude::Altitude;

    // TODO: Test altitude.
}
