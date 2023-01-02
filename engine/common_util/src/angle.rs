// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::serde_util::{F32Visitor, I16Visitor};
use glam::{Vec2, Vec2Swizzles};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::f32::consts::PI;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

pub type AngleRepr = i16;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Angle(pub AngleRepr);

#[allow(dead_code)]
impl Angle {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(AngleRepr::MAX);
    pub const PI: Self = Self(AngleRepr::MAX);
    pub const PI_2: Self = Self(AngleRepr::MAX / 2);

    pub fn new() -> Self {
        Self::ZERO
    }

    pub fn from_atan2(y: f32, x: f32) -> Self {
        Self::from_radians(y.atan2(x))
    }

    #[inline]
    pub fn to_vec(self) -> Vec2 {
        Vec2::from(self.to_radians().sin_cos()).yx()
    }

    #[deprecated]
    pub fn from_vec(vec: Vec2) -> Self {
        Self::from_atan2(vec.y, vec.x)
    }

    #[inline]
    pub fn to_radians(self) -> f32 {
        self.0 as f32 * (PI / Self::PI.0 as f32)
    }

    #[inline]
    pub fn from_radians(radians: f32) -> Self {
        Self((radians * (Self::PI.0 as f32 / PI)) as i32 as AngleRepr)
    }

    /// Like from_radians but angles greater than PI are mapped to PI, and angles less than -PI are
    /// mapped to -PI.
    #[inline]
    pub fn saturating_from_radians(radians: f32) -> Self {
        Self((radians * (Self::PI.0 as f32 / PI)) as AngleRepr)
    }

    pub fn to_degrees(self) -> f32 {
        self.to_radians().to_degrees()
    }

    pub fn from_degrees(degrees: f32) -> Self {
        Self::from_radians(degrees.to_radians())
    }

    /// One revolution is 360 degrees.
    #[inline]
    pub fn from_revolutions(revolutions: f32) -> Self {
        Self((revolutions * (2.0 * AngleRepr::MAX as f32)) as i32 as AngleRepr)
    }

    pub fn abs(self) -> Self {
        if self.0 == AngleRepr::MIN {
            // Don't negate with overflow.
            return Angle::MAX;
        }
        Self(self.0.abs())
    }

    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    pub fn clamp_magnitude(self, max: Self) -> Self {
        if max.0 >= 0 {
            Self(self.0.clamp(-max.0, max.0))
        } else {
            // Clamping to over 180 degrees in either direction, any angle is valid.
            self
        }
    }

    pub fn lerp(self, other: Self, value: f32) -> Self {
        self + (other - self) * value
    }

    /// Increases clockwise with straight up being 0. Output always 0..=359, never 360.
    pub fn to_bearing(self) -> u16 {
        ((Self::PI_2 - self).0 as u16 as u32 * 360 / (u16::MAX as u32 + 1)) as u16
    }

    /// E, NE, SW, etc.
    pub fn to_cardinal(self) -> &'static str {
        let idx =
            ((self.0 as u16).wrapping_add(u16::MAX / 16)) / ((u16::MAX as u32 + 1) / 8) as u16;
        ["E", "NE", "N", "NW", "W", "SW", "S", "SE"][idx as usize]
    }
}

impl Default for Angle {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<Angle> for Vec2 {
    fn from(angle: Angle) -> Self {
        #[allow(deprecated)]
        angle.to_vec()
    }
}

impl From<Vec2> for Angle {
    fn from(vec: Vec2) -> Self {
        #[allow(deprecated)]
        Self::from_vec(vec)
    }
}

impl Add for Angle {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.wrapping_add(other.0))
    }
}

impl AddAssign for Angle {
    fn add_assign(&mut self, other: Self) {
        self.0 = self.0.wrapping_add(other.0);
    }
}

impl Sub for Angle {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.wrapping_sub(other.0))
    }
}

impl SubAssign for Angle {
    fn sub_assign(&mut self, other: Self) {
        self.0 = self.0.wrapping_sub(other.0);
    }
}

impl Neg for Angle {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::ZERO - self
    }
}

impl Mul<f32> for Angle {
    type Output = Self;

    fn mul(self, other: f32) -> Self::Output {
        Self((self.0 as f32 * other) as i32 as AngleRepr)
    }
}

#[cfg(feature = "rand")]
use rand::prelude::*;
#[cfg(feature = "rand")]
impl Distribution<Angle> for rand::distributions::Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Angle {
        Angle(rng.gen())
    }
}

impl fmt::Debug for Angle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} degrees", self.to_degrees())
    }
}

impl Serialize for Angle {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_f32(self.to_radians())
        } else {
            serializer.serialize_i16(self.0)
        }
    }
}

impl<'de> Deserialize<'de> for Angle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer
                .deserialize_f32(F32Visitor)
                .map(Self::from_radians)
        } else {
            deserializer.deserialize_i16(I16Visitor).map(Self)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::angle::Angle;

    #[test]
    fn radians() {
        for i in -1000..1000 {
            let r = (i as f32) / 100.0;
            let a = Angle::from_radians(r);
            let r2 = a.to_radians();
            let a2 = Angle::from_radians(r2);
            assert!((a - a2).to_radians().abs() < 0.0001, "{:?} -> {:?}", a, a2);
        }
    }

    #[test]
    fn serde() {
        for i in -1000..1000 {
            let r = (i as f32) / 100.0;
            let rs = format!("{}", r);
            let a: Angle = serde_json::from_str(&*rs).unwrap();
            let rs2 = serde_json::to_string(&a).unwrap();
            let a2: Angle = serde_json::from_str(&*rs2).unwrap();
            assert!((a - a2).to_radians().abs() < 0.0001, "{:?} -> {:?}", a, a2);
        }
    }

    #[test]
    fn pi() {
        // Just less than PI.
        let rs = "3.141592653589793";
        let a: Angle = serde_json::from_str(rs).unwrap();
        assert_eq!(a, Angle::PI);

        // Greater than PI.
        let rs2 = "3.141689";
        let a2: Angle = serde_json::from_str(rs2).unwrap();
        assert!(a2.to_radians() < -3.0)
    }

    #[test]
    fn unit_vec() {
        let v = Angle::ZERO.to_vec();
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 0.0);

        let v2 = Angle::PI_2.to_vec();
        assert!(v2.x.abs() < 0.001);
        assert_eq!(v2.y, 1.0);
    }

    #[test]
    fn abs() {
        assert_eq!(Angle::from_radians(0.0).abs(), Angle::from_radians(0.0));
        assert_eq!(Angle::from_radians(0.5).abs(), Angle::from_radians(0.5));
        assert_eq!(Angle::from_radians(-0.5).abs(), Angle::from_radians(0.5));
    }

    #[test]
    fn min() {
        assert_eq!(
            Angle::from_radians(0.5).min(Angle::from_radians(0.6)),
            Angle::from_radians(0.5)
        );
        assert_eq!(
            Angle::from_radians(0.5).min(Angle::from_radians(0.4)),
            Angle::from_radians(0.4)
        );
        assert_eq!(
            Angle::from_radians(-0.5).min(Angle::from_radians(0.6)),
            Angle::from_radians(-0.5)
        );
        assert_eq!(
            Angle::from_radians(-0.5).min(Angle::from_radians(0.4)),
            Angle::from_radians(-0.5)
        );
    }

    #[test]
    fn clamp_magnitude() {
        assert_eq!(
            Angle::from_radians(0.5).clamp_magnitude(Angle::from_radians(0.6)),
            Angle::from_radians(0.5)
        );
        assert_eq!(
            Angle::from_radians(0.5).clamp_magnitude(Angle::from_radians(0.4)),
            Angle::from_radians(0.4)
        );
        assert_eq!(
            Angle::from_radians(-0.5).clamp_magnitude(Angle::from_radians(0.6)),
            Angle::from_radians(-0.5)
        );
        assert_eq!(
            Angle::from_radians(-0.5).clamp_magnitude(Angle::from_radians(0.4)),
            Angle::from_radians(-0.4)
        );
    }

    #[test]
    fn to_bearing() {
        assert_eq!(Angle::PI_2.to_bearing(), 0);
        assert_eq!(Angle::PI.to_bearing(), 270);

        for i in 0..i16::MAX {
            let b = Angle(i).to_bearing();
            assert!(b < 360, "{} -> {} >= 360", i, b);
        }
    }

    #[test]
    fn to_cardinal() {
        // Make sure it doesn't panic.
        for i in 0..i16::MAX {
            Angle(i).to_cardinal();
        }

        assert_eq!(Angle::ZERO.to_cardinal(), "E");
        assert_eq!(Angle::PI_2.to_cardinal(), "N");
        assert_eq!(Angle::PI.to_cardinal(), "W");
        assert_eq!(Angle(u16::MAX as i16).to_cardinal(), "E");
    }
}
