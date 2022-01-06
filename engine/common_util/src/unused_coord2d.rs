// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::Vec2;
use std::ops::Mul;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Coord2D<const SCALE: usize, const SIZE: usize>(usize, usize);

/// Any terrain pixel can be represented as a `Coord2D`.
#[allow(dead_code)]
impl<const SCALE: usize, const SIZE: usize> Coord2D<SCALE, SIZE> {
    // Offset to convert between signed coordinates to unsigned.
    const OFFSET: isize = (SIZE / 2) as isize;

    pub fn from_position(v: Vec2) -> Option<Self> {
        let v = v.mul(1.0 / SCALE as f32);
        Self::from_scaled_position(v)
    }

    /// Converts a position to the nearest valid `Coord2D`.
    fn saturating_from_position(mut pos: Vec2) -> Self {
        pos *= 1.0 / (SCALE as f32);
        pos += Self::OFFSET as f32;
        let x = (pos.x as i64).clamp(0, (SIZE - 1) as i64) as usize;
        let y = (pos.y as i64).clamp(0, (SIZE - 1) as i64) as usize;
        Self(x, y)
    }

    fn from_scaled_position(v: Vec2) -> Option<Self> {
        let coord = unsafe {
            Self(
                (v.x.to_int_unchecked::<isize>() + Self::OFFSET) as usize,
                (v.y.to_int_unchecked::<isize>() + Self::OFFSET) as usize,
            )
        };

        if coord.0 >= SIZE || coord.1 >= SIZE {
            None
        } else {
            Some(coord)
        }
    }

    pub fn corner(&self) -> Vec2 {
        Vec2::new(
            (self.0 as isize - Self::OFFSET) as f32,
            (self.1 as isize - Self::OFFSET) as f32,
        )
        .mul(SCALE as f32)
    }
}

impl<U, const SCALE: usize, const SIZE: usize> From<(U, U)> for Coord2D<SCALE, SIZE>
where
    U: Into<u64>,
{
    fn from(x: (U, U)) -> Self {
        Self(x.0.into() as usize, x.1.into() as usize)
    }
}
