// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::angle::Angle;
use glam::Vec2;
use rand::prelude::ThreadRng;
use rand::Rng;
use std::ops::Range;

/// map_ranges linearly maps a number from one range to another, optionally clamping to the new range.
/// If clamp is true, the new range must obey end >= start.
pub fn map_ranges(number: f32, old: Range<f32>, new: Range<f32>, clamp_to_range: bool) -> f32 {
    let old_range = old.end - old.start;
    let new_range = new.end - new.start;
    let number_normalized = (number - old.start) / old_range;
    let mut mapped = new.start + number_normalized * new_range;
    if clamp_to_range {
        debug_assert!(
            new.start <= new.end,
            "map_ranges requires start < end if clamp=true"
        );
        mapped = mapped.clamp(new.start, new.end);
    }
    mapped
}

/// level_to_score converts a boat level to a score required to upgrade to it.
pub fn level_to_score(level: u8) -> u32 {
    ((level as u32).pow(2) - 1) * 10
}

/// lerp linearly interpolates (and possibly extrapolates) from start to end, based on amount.
#[inline]
pub fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    amount.mul_add(end, (-amount).mul_add(start, start))
}

/// Samples a point from a circle with the given radius.
pub fn gen_radius(rng: &mut ThreadRng, radius: f32) -> Vec2 {
    rng.gen::<Angle>().to_vec() * (rng.gen::<f32>().sqrt() * radius)
}
