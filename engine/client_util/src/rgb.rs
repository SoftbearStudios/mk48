// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{Vec2, Vec2Swizzles, Vec3, Vec4};

/// All equal RGB to Vec3.
/// /// See rgb_array.
pub fn gray(v: u8) -> Vec3 {
    Vec3::splat(v as f32 * (1.0 / 255.0))
}

/// RGB color to Vec3.
/// See rgb_array.
pub fn rgb(r: u8, g: u8, b: u8) -> Vec3 {
    rgb_array([r, g, b])
}

/// RGB hex color to Vec3.
/// See rgb_array.
pub fn rgb_hex(hex: u32) -> Vec3 {
    debug_assert!(hex <= 0xffffff, "rgb has no alpha");
    let [_, r, g, b] = hex.to_be_bytes();
    rgb_array([r, g, b])
}

/// RGB components mapped from 0-255 to 0.0-1.0.
pub fn rgb_array(rgb: [u8; 3]) -> Vec3 {
    Vec3::from(rgb.map(|v| v as f32 * (1.0 / 255.0)))
}

/// All equal RGB and an A to Vec4.
/// See rgba_array.
pub fn gray_a(v: u8, a: u8) -> Vec4 {
    (Vec2::new(v as f32, a as f32) * (1.0 / 255.0)).xxxy()
}

/// RGBA color to Vec4.
/// See rgba_array.
pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Vec4 {
    rgba_array([r, g, b, a])
}

/// RGBA hex color to Vec4.
/// See rgba_array.
pub fn rgba_hex(hex: u32) -> Vec4 {
    rgba_array(hex.to_be_bytes())
}

/// RGBA components mapped from 0-255 to 0.0-1.0.
pub fn rgba_array(rgba: [u8; 4]) -> Vec4 {
    Vec4::from(rgba.map(|v| v as f32 * (1.0 / 255.0)))
}
