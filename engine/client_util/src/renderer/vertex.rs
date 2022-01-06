// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::attribute::{Attribs, Attribute};
pub use engine_macros::Vertex;
use glam::{Vec2, Vec4};
use std::mem::size_of;

/// Vertex is any vertex data consisting of floats.
pub trait Vertex: Sized {
    fn size() -> usize {
        size_of::<Self>()
    }

    fn floats() -> usize {
        Self::size() / 4
    }

    // TODO create #[derive(Vertex)]
    fn bind_attribs(attribs: &mut Attribs<Self>);
}

/// Vec2 stores a vertex with (only) a given position.
impl Vertex for Vec2 {
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        Vec2::bind_attrib(attribs);
    }
}

/// PosUvAlpha stores a vertex with (only) a given position, texture coordinate, and alpha.
#[repr(C)]
pub struct PosUvAlpha {
    pub pos: Vec2,
    pub uv: Vec2,
    pub alpha: f32,
}

impl Vertex for PosUvAlpha {
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        Vec2::bind_attrib(attribs);
        Vec2::bind_attrib(attribs);
        f32::bind_attrib(attribs);
    }
}

/// PosColor stores a vertex with (only) a given position and color.
#[repr(C)]
pub struct PosColor {
    pub pos: Vec2,
    // This is normally 16 byte aligned (breaking attribute size assertion), but not with glam's scalar-math feature enabled.
    pub color: Vec4,
}

impl Vertex for PosColor {
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        Vec2::bind_attrib(attribs);
        Vec4::bind_attrib(attribs);
    }
}
