// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::angle::Angle;
use glam::Vec2;
use rand::prelude::ThreadRng;
use rand::Rng;
use std::ops::Range;
use std::sync::Arc;

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
pub const fn level_to_score(level: u8) -> u32 {
    // For reference, https://www.desmos.com/calculator/8cwxdws7fp
    ((level as u32).pow(2) + 2u32.pow(level.saturating_sub(3) as u32) - 2) * 10
}

/// respawn_score returns how much score is kept when a boat dies.
pub fn respawn_score(score: u32) -> u32 {
    // Lose 1/2 score if you die.
    // Cap so can't get max level right away.
    (score / 2).min(level_to_score(4))
}

/// respawn_score returns how much score a boat gets from a kill.
pub fn kill_score(score: u32) -> u32 {
    10 + score / 4
}

/// respawn_score returns how much score a boat gets from a ramming kill.
pub fn ram_score(score: u32) -> u32 {
    kill_score(score) / 2
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

/// returns a float in range [0, 1) based on n.
pub fn hash_u32_to_f32(n: u32) -> f32 {
    let hash_size = 64;
    (n & (hash_size - 1)) as f32 * (1.0 / hash_size as f32)
}

/// make_mut_slice derives a mutable slice from an Arc, cloning the Arc if necessary.
pub fn make_mut_slice<T: Clone>(arc: &mut Arc<[T]>) -> &mut [T] {
    let mut_ref = unsafe { &mut *(arc as *mut Arc<[T]>) };

    match Arc::get_mut(mut_ref) {
        Some(x) => x,
        None => {
            *arc = arc.iter().cloned().collect();
            Arc::get_mut(arc).unwrap()
        }
    }
}
