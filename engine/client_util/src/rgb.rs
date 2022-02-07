// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{Vec3, Vec4};

/// RGB components from 0-255 to 0.0-1.0
pub fn rgb(r: u8, b: u8, g: u8) -> Vec3 {
    Vec3::new(r as f32, b as f32, g as f32) * (1.0 / 255.0)
}

/// RGBA components from 0-255 to 0.0-1.0
pub fn rgba(r: u8, b: u8, g: u8, a: u8) -> Vec4 {
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
