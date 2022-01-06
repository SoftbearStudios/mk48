// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::serde_util::{F32Visitor, U16Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::*;
use std::time::Duration;

pub type TicksRepr = u16;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Ticks(pub TicksRepr);

/// Ticks efficiently stores an unsigned duration.
impl Ticks {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);
    pub const MAX: Self = Self(TicksRepr::MAX);
    pub const RATE: Ticks = Ticks(10);
    pub const PERIOD: f32 = 1.0 / (Self::RATE.0 as f32);

    /// REGEN_DAMAGE controls how long it takes to regenerate one unit of damage.
    const REGEN_DAMAGE: Self = Self((Self::RATE.0 * 60) as TicksRepr);

    /// Converts fractional seconds to a duration, which can be quite lossy.
    pub fn from_secs(secs: f32) -> Self {
        Self((secs * Self::RATE.0 as f32) as TicksRepr)
    }

    /// Returns the duration as fractional seconds.
    pub fn to_secs(self) -> f32 {
        self.0 as f32 * Self::PERIOD
    }

    /// Converts the duration in ticks to a formal `Duration`.
    pub fn to_duration(self) -> Duration {
        Duration::from_secs_f32(self.to_secs())
    }

    /// from_damage returns the amount of Ticks required to regenerate a given amount of damage.
    /// TODO: Eliminate the concept of damage entirely, and only use Ticks.
    pub fn from_damage(damage: f32) -> Self {
        Self::from_secs(damage * Self::REGEN_DAMAGE.to_secs())
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
}

impl Default for Ticks {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Add for Ticks {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl AddAssign for Ticks {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Sub for Ticks {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0 - other.0)
    }
}

impl SubAssign for Ticks {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl Mul for Ticks {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self(self.0 * other.0)
    }
}

impl Mul<f32> for Ticks {
    type Output = Self;

    fn mul(self, other: f32) -> Self::Output {
        Self((self.0 as f32 * other) as TicksRepr)
    }
}

impl MulAssign<f32> for Ticks {
    fn mul_assign(&mut self, other: f32) {
        *self = *self * other;
    }
}

impl Div for Ticks {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Self(self.0 / other.0)
    }
}

impl Rem for Ticks {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self(self.0 % rhs.0)
    }
}

impl fmt::Debug for Ticks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} seconds", self.to_secs())
    }
}

impl Serialize for Ticks {
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

impl<'de> Deserialize<'de> for Ticks {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer
                .deserialize_f32(F32Visitor)
                .map(Ticks::from_secs)
        } else {
            deserializer.deserialize_u16(U16Visitor).map(Ticks)
        }
    }
}
