// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::serde_util::{F32Visitor, U16Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::*;
use std::time::Duration;

pub type TicksRepr = u16;

/// Ticks, generic over frequency. Each game should define a type alias with a specific frequency.
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GenTicks<const FREQUENCY_HZ: TicksRepr>(pub TicksRepr);

/// Ticks efficiently stores an unsigned duration.
impl<const FREQUENCY_HZ: TicksRepr> GenTicks<FREQUENCY_HZ> {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);
    pub const MAX: Self = Self(TicksRepr::MAX);
    pub const FREQUENCY_HZ: Self = GenTicks(FREQUENCY_HZ);
    pub const PERIOD_SECS: f32 = 1.0 / (Self::FREQUENCY_HZ.0 as f32);

    /// Converts fractional seconds to a duration, which can be quite lossy.
    pub fn from_secs(secs: f32) -> Self {
        Self((secs * Self::FREQUENCY_HZ.0 as f32) as TicksRepr)
    }

    /// Converts whole seconds to a duration.
    pub const fn from_whole_secs(secs: TicksRepr) -> Self {
        debug_assert!(
            secs.checked_mul(Self::FREQUENCY_HZ.0).is_some(),
            "from_whole_secs overflow"
        );
        Self(secs.saturating_mul(Self::FREQUENCY_HZ.0))
    }

    /// Converts whole millis to a duration.
    pub const fn from_whole_millis(secs: u32) -> Self {
        let scaled = secs * Self::FREQUENCY_HZ.0 as u32 / 1000;
        if scaled > TicksRepr::MAX as u32 {
            debug_assert!(false, "from_whole_millis overflow");
            Self::MAX
        } else {
            Self(scaled as TicksRepr)
        }
    }

    /// Returns some absolute number of ticks.
    pub const fn from_repr(ticks: TicksRepr) -> Self {
        Self(ticks)
    }

    /// Returns the duration as fractional seconds.
    pub fn to_secs(self) -> f32 {
        self.0 as f32 * Self::PERIOD_SECS
    }

    /// Returns the duration as whole seconds (floored).
    pub fn to_whole_secs(self) -> TicksRepr {
        self.0 / Self::FREQUENCY_HZ.0
    }

    /// Converts the duration in ticks to a formal `Duration`.
    pub fn to_duration(self) -> Duration {
        Duration::from_secs_f32(self.to_secs())
    }

    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub fn wrapping_add(self, rhs: Self) -> Self {
        Self(self.0.wrapping_add(rhs.0))
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    pub fn next(self) -> Self {
        self.wrapping_add(Self::ONE)
    }

    /// Returns true if self % period == Self::ZERO.
    pub fn every(self, period: Self) -> bool {
        self % period == Self::ZERO
    }
}

impl<const FREQUENCY_HZ: TicksRepr> Add for GenTicks<FREQUENCY_HZ> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl<const FREQUENCY_HZ: TicksRepr> AddAssign for GenTicks<FREQUENCY_HZ> {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl<const FREQUENCY_HZ: TicksRepr> Sub for GenTicks<FREQUENCY_HZ> {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl<const FREQUENCY_HZ: TicksRepr> SubAssign for GenTicks<FREQUENCY_HZ> {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl<const FREQUENCY_HZ: TicksRepr> Mul for GenTicks<FREQUENCY_HZ> {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self(self.0 * other.0)
    }
}

impl<const FREQUENCY_HZ: TicksRepr> Mul<f32> for GenTicks<FREQUENCY_HZ> {
    type Output = Self;

    fn mul(self, other: f32) -> Self::Output {
        Self((self.0 as f32 * other) as TicksRepr)
    }
}

impl<const FREQUENCY_HZ: TicksRepr> MulAssign<f32> for GenTicks<FREQUENCY_HZ> {
    fn mul_assign(&mut self, other: f32) {
        *self = *self * other;
    }
}

impl<const FREQUENCY_HZ: TicksRepr> Div for GenTicks<FREQUENCY_HZ> {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Self(self.0 / other.0)
    }
}

impl<const FREQUENCY_HZ: TicksRepr> Rem for GenTicks<FREQUENCY_HZ> {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self(self.0 % rhs.0)
    }
}

impl<const FREQUENCY_HZ: TicksRepr> fmt::Debug for GenTicks<FREQUENCY_HZ> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} seconds", self.to_secs())
    }
}

impl<const FREQUENCY_HZ: TicksRepr> Serialize for GenTicks<FREQUENCY_HZ> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_f32(self.to_secs())
        } else {
            serializer.serialize_u16(self.0)
        }
    }
}

impl<'de, const FREQUENCY_HZ: TicksRepr> Deserialize<'de> for GenTicks<FREQUENCY_HZ> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer
                .deserialize_f32(F32Visitor)
                .map(GenTicks::from_secs)
        } else {
            deserializer.deserialize_u16(U16Visitor).map(GenTicks)
        }
    }
}
