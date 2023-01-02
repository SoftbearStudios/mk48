// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ticks::{Ticks, TicksRepr};
use common_util::range::map_ranges;
use core_protocol::serde_util::{F32Visitor, I8Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

type AltitudeRepr = i8;

#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Altitude(pub AltitudeRepr);

#[allow(dead_code)]
impl Altitude {
    pub const ZERO: Self = Self(0);
    pub const UNIT: Self = Self(1);
    pub const MIN: Self = Self(AltitudeRepr::MIN);
    pub const MAX: Self = Self(AltitudeRepr::MAX);
    const SCALE_INT: i16 = 2;
    const SCALE: f32 = Self::SCALE_INT as f32;

    /// Altitudes within this margin are considered to be overlapping.
    pub const OVERLAP_MARGIN: Altitude = Altitude(AltitudeRepr::MAX / 4);

    /// Altitudes within this margin are considered to be overlapping for cases in which lack of guidance
    /// creates an unbalanced experience i.e. battleships and their non-homing torpedoes vs deep subs.
    pub const SPECIAL_OVERLAP_MARGIN: Altitude = Altitude(AltitudeRepr::MAX / 2);

    pub fn new() -> Self {
        Self::ZERO
    }

    pub fn to_meters(self) -> f32 {
        self.0 as f32 * Self::SCALE
    }

    pub fn from_meters(meters: f32) -> Self {
        Self((meters * (1.0 / Self::SCALE)) as AltitudeRepr)
    }

    pub const fn from_whole_meters(meters: i16) -> Self {
        let scaled = meters / Self::SCALE_INT;
        // clamp isn't const :(
        if scaled < AltitudeRepr::MIN as i16 {
            Self::MIN
        } else if scaled > AltitudeRepr::MAX as i16 {
            Self::MAX
        } else {
            Self(scaled as AltitudeRepr)
        }
    }

    // The u8 is interpreted as 0-255 meaning MIN-MAX.
    pub const fn from_u8(alt: u8) -> Self {
        Self((alt as i16 - 128) as AltitudeRepr)
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
        ) as AltitudeRepr)
    }

    /// to_norm returns self in the range [-1, 1] where 0.0 is sea level.
    pub fn to_norm(self) -> f32 {
        map_ranges(
            self.0 as f32,
            Self::MIN.0 as f32..Self::MAX.0 as f32,
            -1.0..1.0,
            false,
        )
    }

    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0) as AltitudeRepr)
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0) as AltitudeRepr)
    }

    /// Returns self, clamped between min and max (max >= min or will panic).
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self(self.0.clamp(min.0, max.0))
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

    /// Returns true if below surface.
    pub fn is_submerged(self) -> bool {
        self < Self::ZERO
    }

    /// Returns true if above zero.
    pub fn is_airborne(self) -> bool {
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

impl Add for Altitude {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.saturating_add(other.0))
    }
}

impl AddAssign for Altitude {
    fn add_assign(&mut self, other: Self) {
        self.0 = self.0.saturating_add(other.0);
    }
}

impl Sub for Altitude {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.saturating_sub(other.0))
    }
}

impl SubAssign for Altitude {
    fn sub_assign(&mut self, other: Self) {
        self.0 = self.0.saturating_sub(other.0);
    }
}

impl Neg for Altitude {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::ZERO - self
    }
}

impl Mul<f32> for Altitude {
    type Output = Self;

    fn mul(self, other: f32) -> Self::Output {
        Self((self.0 as f32 * other) as AltitudeRepr)
    }
}

impl Mul<Ticks> for Altitude {
    type Output = Self;

    fn mul(self, other: Ticks) -> Self::Output {
        debug_assert!(other.0 < AltitudeRepr::MAX as TicksRepr);
        Self((self.0.saturating_mul(other.0 as AltitudeRepr)) as AltitudeRepr)
    }
}

impl fmt::Debug for Altitude {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_meters())
    }
}

impl Serialize for Altitude {
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

impl<'de> Deserialize<'de> for Altitude {
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
