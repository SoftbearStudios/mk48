// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::ops::Range;

/// map_ranges linearly maps a number from one range to another, optionally clamping to the new range.
#[inline]
pub fn map_ranges(
    number: f32,
    mut old: Range<f32>,
    mut new: Range<f32>,
    clamp_to_range: bool,
) -> f32 {
    if clamp_to_range && new.end < new.start {
        new = new.end..new.start;
        old = old.end..old.start;
    }
    map_ranges_fast(number, old, new, clamp_to_range, clamp_to_range)
}

// Faster version of map_ranges.
// Make sure all inputs are constant for fastest execution.
// Can optimize to 1 FMA instruction + 1 MAX for clamp_low + 1 MIN for clamp_high.
#[inline]
pub fn map_ranges_fast(
    number: f32,
    old: Range<f32>,
    new: Range<f32>,
    clamp_low: bool,
    clamp_high: bool,
) -> f32 {
    let old_range = old.end - old.start;
    let new_range = new.end - new.start;
    let mul: f32 = new_range / old_range;
    let add: f32 = -old.start * mul + new.start;

    let mut result = if cfg!(target_feature = "fma") {
        number.mul_add(mul, add)
    } else {
        number * mul + add
    };

    if clamp_low || clamp_high {
        assert!(new_range >= 0.0, "requires new.start < new.end if clamping");
    }
    if clamp_low {
        result = result.max(new.start)
    }
    if clamp_high {
        result = result.min(new.end)
    }
    result
}

/// lerp linearly interpolates (and possibly extrapolates) from start to end, based on amount.
/// It uses FMA instruction if available.
#[inline]
pub fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    let mul_add = if cfg!(target_feature = "fma") {
        f32::mul_add
    } else {
        |a, b, c| a * b + c
    };
    mul_add(amount, end, mul_add(-amount, start, start))
}

#[cfg(feature = "rand")]
use rand::Rng;
/// Samples a point from a circle with the given radius.
#[cfg(feature = "rand")]
pub fn gen_radius(rng: &mut impl Rng, radius: f32) -> glam::Vec2 {
    rng.gen::<crate::angle::Angle>().to_vec() * (rng.gen::<f32>().sqrt() * radius)
}

#[cfg(test)]
mod tests {
    use crate::range::{map_ranges, map_ranges_fast};

    #[test]
    fn test_map_range() {
        assert_eq!(map_ranges(1.5, 1.0..2.0, -4.0..-8.0, false), -6.0);
        assert_eq!(map_ranges(1.5, 1.0..2.0, -4.0..-8.0, true), -6.0);
        assert_eq!(map_ranges(1.5, 2.0..1.0, -8.0..-4.0, false), -6.0);
        assert_eq!(map_ranges(1.5, 2.0..1.0, -8.0..-4.0, true), -6.0);
        assert_eq!(map_ranges(10.0, 0.0..1.0, 2.0..3.0, true), 3.0);
        assert_eq!(map_ranges(10.0, 1.0..0.0, 3.0..2.0, true), 3.0);
    }

    #[test]
    fn test_map_ranges_fast() {
        assert_eq!(
            map_ranges_fast(1.5, 1.0..2.0, -4.0..-8.0, false, false),
            -6.0
        );
        assert_eq!(map_ranges_fast(10.0, 0.0..1.0, 2.0..3.0, true, true), 3.0);
    }
}
