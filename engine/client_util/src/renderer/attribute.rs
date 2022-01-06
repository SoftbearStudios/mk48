// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::vertex::Vertex;
use glam::{Vec2, Vec3, Vec4};
use std::marker::PhantomData;
use std::mem::{align_of, size_of};
use web_sys::WebGlRenderingContext as Gl;

/// Vertex attributes.
pub struct Attribs<'a, V: Vertex> {
    gl: &'a Gl,
    bytes: u32,
    index: u32,
    vertex: PhantomData<V>,
}

impl<'a, V: Vertex> Attribs<'a, V> {
    pub(crate) fn new(gl: &'a Gl) -> Self {
        Self {
            gl,
            bytes: 0,
            index: 0,
            vertex: PhantomData,
        }
    }

    fn attrib(&mut self) -> u32 {
        let i = self.index;
        self.gl.enable_vertex_attrib_array(i);
        self.index += 1;
        i
    }

    fn offset(&mut self, bytes: usize) -> i32 {
        let b = self.bytes;
        self.bytes += bytes as u32;
        b as i32
    }

    fn floats(&mut self, count: usize) {
        self.gl.vertex_attrib_pointer_with_i32(
            self.attrib(),
            count as i32,
            Gl::FLOAT,
            false,
            V::size() as i32,
            self.offset(count * size_of::<f32>()),
        );
    }
}

impl<'a, V: Vertex> Drop for Attribs<'a, V> {
    fn drop(&mut self) {
        // Make sure all attributes were added.
        assert_eq!(self.bytes as usize, V::size(), "attributes don't add up");
        // Check safety of slice::from_raw_parts.
        assert_eq!(align_of::<V>(), 4, "alignment must be 4 bytes")
    }
}

pub trait Attribute {
    fn bind_attrib<V: Vertex>(attribs: &mut Attribs<V>);
}

macro_rules! impl_attribute_floats {
    ($a: ty, $floats: literal) => {
        impl Attribute for $a {
            fn bind_attrib<V: Vertex>(attribs: &mut Attribs<V>) {
                attribs.floats($floats);
            }
        }
    };
}

impl_attribute_floats!(f32, 1);
impl_attribute_floats!(Vec2, 2);
impl_attribute_floats!(Vec3, 3);
impl_attribute_floats!(Vec4, 4);
