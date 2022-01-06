// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{vec3, vec4, Vec3, Vec4};

/// RGB components from 0-255 to 0.0-1.0
pub fn rgb(r: u8, b: u8, g: u8) -> Vec3 {
    vec3(r as f32, b as f32, g as f32) * (1.0 / 255.0)
}

/// RGBA components from 0-255 to 0.0-1.0
pub fn rgba(r: u8, b: u8, g: u8, a: u8) -> Vec4 {
    vec4(r as f32, b as f32, g as f32, a as f32) * (1.0 / 255.0)
}

/// RGB component from 0-255 to 0.0-1.0
pub fn gray(v: u8) -> Vec3 {
    Vec3::splat(v as f32 * (1.0 / 255.0))
}
