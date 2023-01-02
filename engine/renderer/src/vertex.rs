// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::attribs::Attribs;
use bytemuck::Pod;
use glam::*;

#[doc(hidden)]
pub use bytemuck::{Pod as __Pod, Zeroable as __Zeroable};
pub use engine_macros::Vertex;

/// Any data consisting of [`prim@f32`]s. You can derive it on a struct with
/// [`derive_vertex`][`crate::derive_vertex`].
pub trait Vertex: Pod {
    #[doc(hidden)]
    fn bind_attribs(attribs: &mut Attribs);
}

/// For easily deriving vertex and friends. Unfortunatly requires putting `bytemuck = "1.9"` in
/// your `Cargo.toml`.
#[macro_export]
macro_rules! derive_vertex {
    ($s:item) => {
        #[derive(Copy, Clone, $crate::Vertex, $crate::__Pod, $crate::__Zeroable)]
        #[repr(C)]
        $s
    };
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

// These are normally 16 byte aligned (breaking derive Pod) but not with glam's scalar-math feature.
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
