// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::gl::*;
use crate::vertex::Vertex;
use std::mem::size_of;

/// For describing [`Vertex`] attributes to shaders. Not extensible right now.
pub struct Attribs<'a> {
    gl: &'a Gl,
    aia: Option<&'a Aia>,
    bytes: u32,
    index: u32,
    size: usize,
}

impl<'a> Attribs<'a> {
    pub(crate) fn new<V: Vertex>(gl: &'a Gl) -> Self {
        Self {
            gl,
            aia: None,
            bytes: 0,
            index: 0,
            size: size_of::<V>(),
        }
    }

    pub(crate) fn new_instanced<V: Vertex>(gl: &'a Gl, aia: &'a Aia, previous: Self) -> Self {
        let index = previous.index;
        Self {
            gl,
            aia: Some(aia),
            bytes: 0,
            index,
            size: size_of::<V>(),
        }
    }

    fn attrib(&mut self) -> u32 {
        let i = self.index;
        self.index += 1;

        self.gl.enable_vertex_attrib_array(i);
        if let Some(aia) = self.aia {
            aia.vertex_attrib_divisor_angle(i, 1);
        }
        i
    }

    fn offset(&mut self, bytes: usize) -> i32 {
        let b = self.bytes;
        self.bytes += bytes as u32;
        b as i32
    }

    pub(crate) fn floats(&mut self, count: usize) {
        self.gl.vertex_attrib_pointer_with_i32(
            self.attrib(),
            count as i32,
            Gl::FLOAT,
            false,
            self.size as i32,
            self.offset(count * size_of::<f32>()),
        );
    }
}

impl<'a> Drop for Attribs<'a> {
    fn drop(&mut self) {
        // Make sure all attributes were added.
        assert_eq!(self.bytes as usize, self.size, "attributes don't add up");
    }
}
