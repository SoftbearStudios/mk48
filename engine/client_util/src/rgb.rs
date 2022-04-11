// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{Vec3, Vec4};
use std::convert::From;

pub struct Rgba {
    value: [f32; 4],
}

impl Rgba {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        let normalize = |value| (value as f32) * (1.0 / 255.0);
        Self {
            value: [normalize(r), normalize(g), normalize(b), normalize(a)],
        }
    }

    /// RGB component from 0-255 to 0.0-1.0
    pub fn gray(value: u8) -> Self {
        [value, value, value, 255 as u8].into()
    }
}

impl From<Vec4> for Rgba {
    fn from(value: Vec4) -> Self {
        Self {
            value: [value.x, value.y, value.z, value.w],
        }
    }
}

impl From<[u8; 4]> for Rgba {
    fn from(value: [u8; 4]) -> Self {
        Rgba::new(value[0], value[1], value[2], value[3])
    }
}

impl From<u32> for Rgba {
    fn from(value: u32) -> Self {
        let r = ((value >> 24) & 255) as u8;
        let g = ((value >> 16) & 255) as u8;
        let b = ((value >> 8) & 255) as u8;
        let a = (value & 255) as u8;

        Rgba::new(r, g, b, a)
    }
}

impl Into<[f32; 4]> for Rgba {
    fn into(self) -> [f32; 4] {
        self.value
    }
}

impl Into<Vec3> for Rgba {
    fn into(self) -> Vec3 {
        Vec3::new(self.value[0], self.value[1], self.value[2])
    }
}

impl Into<Vec4> for Rgba {
    fn into(self) -> Vec4 {
        Vec4::new(self.value[0], self.value[1], self.value[2], self.value[3])
    }
}

/**
 * Previous implementation below!
 */

/// RGB components from 0-255 to 0.0-1.0
pub fn rgb(r: u8, b: u8, g: u8) -> Vec3 {
    Vec3::new(r as f32, b as f32, g as f32) * (1.0 / 255.0)
}

/// RGBA components from 0-255 to 0.0-1.0
pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Vec4 {
    rgba_array([r, g, b, a])
}

/// RGBA components from 0-255 to 0.0-1.0
pub fn rgba_array(rgba: [u8; 4]) -> Vec4 {
    Vec4::new(
        rgba[0] as f32,
        rgba[1] as f32,
        rgba[2] as f32,
        rgba[3] as f32,
    ) * (1.0 / 255.0)
}

/// RGB component from 0-255 to 0.0-1.0
pub fn gray(v: u8) -> Vec3 {
    Vec3::splat(v as f32 * (1.0 / 255.0))
}
