// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{Vec2, Vec2Swizzles, Vec3, Vec4};

/// All equal RGB to [`Vec3`].
pub fn gray(v: u8) -> Vec3 {
    Vec3::splat(convert(v))
}

/// All equal RGB and an A to [`Vec4`].
pub fn gray_a(v: u8, a: u8) -> Vec4 {
    Vec2::new(convert(v), convert_a(a)).xxxy()
}

/// RGB components to [`Vec3`].
pub fn rgb(r: u8, g: u8, b: u8) -> Vec3 {
    rgb_array([r, g, b])
}

/// RGB array to [`Vec3`].
pub fn rgb_array(rgb: [u8; 3]) -> Vec3 {
    Vec3::from(rgb.map(convert))
}

/// RGB hex to [`Vec3`].
pub fn rgb_hex(hex: u32) -> Vec3 {
    debug_assert!(hex <= 0xffffff, "rgb has no alpha");
    let [_, r, g, b] = hex.to_be_bytes();
    rgb_array([r, g, b])
}

/// RGBA components to [`Vec4`].
pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Vec4 {
    rgba_array([r, g, b, a])
}

/// RGBA array to [`Vec4`].
pub fn rgba_array(rgba: [u8; 4]) -> Vec4 {
    Vec4::new(
        convert(rgba[0]),
        convert(rgba[1]),
        convert(rgba[2]),
        convert_a(rgba[3]),
    )
}

/// RGBA array to css color [`String`].
pub fn rgba_array_to_css(rgba: [u8; 4]) -> String {
    format!("#{:08x}", u32::from_be_bytes(rgba))
}

/// RGBA hex to [`Vec4`].
pub fn rgba_hex(hex: u32) -> Vec4 {
    rgba_array(hex.to_be_bytes())
}

// Converts a red, green or blue u8 to an f32.
fn convert(v: u8) -> f32 {
    #[cfg(not(feature = "srgb"))]
    return v as f32 * (1.0 / 255.0);
    #[cfg(feature = "srgb")]
    srgb::gamma::expand_u8(v)
}

/// Converts an alpha u8 to an f32.
fn convert_a(v: u8) -> f32 {
    v as f32 * (1.0 / 255.0)
}
