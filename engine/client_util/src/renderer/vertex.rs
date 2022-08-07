// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::attribs::Attribs;
use bytemuck::{Pod, Zeroable};
pub use engine_macros::Vertex;
use glam::*;

/// Vertex is any vertex data consisting of floats (or rounded to 4 bytes).
pub trait Vertex: Pod {
    fn bind_attribs(attribs: &mut Attribs);
}

#[derive(Copy, Clone, Pod, Zeroable, Vertex)]
#[repr(C)]
pub struct PosUv {
    pub pos: Vec2,
    pub uv: Vec2,
}
/// PosUvAlpha stores a vertex with (only) a given position, texture coordinate, and alpha.
#[derive(Copy, Clone, Pod, Zeroable, Vertex)]
#[repr(C)]
pub struct PosUvAlpha {
    pub pos: Vec2,
    pub uv: Vec2,
    pub alpha: f32,
}

/// PosColor stores a vertex with (only) a given position and color.
#[derive(Copy, Clone, Pod, Zeroable, Vertex)]
#[repr(C)]
pub struct PosColor {
    pub pos: Vec2,
    // This is normally 16 byte aligned (breaking derive Pod), but not with glam's scalar-math feature enabled.
    pub color: Vec4,
}

macro_rules! impl_vertex_floats {
    ($a: ty, $floats: literal) => {
        impl Vertex for $a {
            fn bind_attribs(attribs: &mut Attribs) {
                attribs.floats($floats);
            }
        }
    };
}

impl_vertex_floats!(f32, 1);
impl_vertex_floats!(Vec2, 2);
impl_vertex_floats!(Vec3, 3);
impl_vertex_floats!(Vec4, 4);
impl_vertex_floats!(Mat2, 4);

impl Vertex for Mat3 {
    fn bind_attribs(attribs: &mut Attribs) {
        for _ in 0..3 {
            attribs.floats(3);
        }
    }
}

impl Vertex for Mat4 {
    fn bind_attribs(attribs: &mut Attribs) {
        for _ in 0..4 {
            attribs.floats(4);
        }
    }
}
