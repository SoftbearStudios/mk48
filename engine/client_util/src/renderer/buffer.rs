// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::attribs::Attribs;
use crate::renderer::index::Index;
use crate::renderer::vertex::Vertex;
use bytemuck::Pod;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::mem::size_of;
use web_sys::{
    AngleInstancedArrays as Aia, OesVertexArrayObject as OesVAO, WebGlBuffer,
    WebGlRenderingContext as Gl, WebGlVertexArrayObject,
};

pub type Quad<I> = [I; 4];

/// MeshBuffer allows building a mesh in RAM.
#[derive(Debug)]
pub struct MeshBuffer<V: Vertex, I: Index = u16> {
    pub vertices: Vec<V>,
    pub indices: Vec<I>,
    default_indices: bool,
}

impl<V: Vertex, I: Index> Default for MeshBuffer<V, I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Vertex, I: Index> MeshBuffer<V, I> {
    /// Create an empty mesh buffer.
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            default_indices: false,
        }
    }

    /// Returns if there are no vertices.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Pushes a single Quad which indexes 4 vertices.
    pub fn push_quad(&mut self, quad: Quad<I>) {
        self.indices
            .extend_from_slice(&[quad[0], quad[1], quad[2], quad[1], quad[3], quad[2]]);
    }

    /// Pushes a point for every vertex.
    pub fn push_default_points(&mut self) {
        assert!(self.indices.is_empty());
        self.default_indices = true;
    }

    /// Pushes a triangle every 3 vertices.
    #[allow(unused)]
    pub fn push_default_triangles(&mut self) {
        assert_eq!(self.vertices.len() % 3, 0);
        self.push_default_points()
    }

    /// Pushes a quad for every 4 vertices.
    pub fn push_default_quads(&mut self) {
        assert!(self.indices.is_empty());

        let n = self.vertices.len();
        assert_eq!(n % 4, 0);
        let quads = n / 4;

        for quad in 0..quads {
            let i = quad * 4;
            self.push_quad([
                I::from_usize(i),
                I::from_usize(i + 1),
                I::from_usize(i + 2),
                I::from_usize(i + 3),
            ]);
        }
    }

    /// Clears all vertices and indices, but preserves their allocations for reuse.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.default_indices = false;
    }
}

pub(crate) enum GpuBufferType {
    Array,
    Element,
}

impl GpuBufferType {
    pub(crate) const fn to(self) -> bool {
        match self {
            Self::Array => true,
            Self::Element => false,
        }
    }

    const fn fr(v: bool) -> Self {
        match v {
            true => Self::Array,
            false => Self::Element,
        }
    }

    const fn target(self) -> u32 {
        match self {
            Self::Array => Gl::ARRAY_BUFFER,
            Self::Element => Gl::ELEMENT_ARRAY_BUFFER,
        }
    }

    const fn parameter(self) -> u32 {
        match self {
            Self::Array => Gl::ARRAY_BUFFER_BINDING,
            Self::Element => Gl::ELEMENT_ARRAY_BUFFER_BINDING,
        }
    }
}

pub(crate) struct GpuBuffer<E: Pod, const B: bool> {
    elements: WebGlBuffer,
    length: u32,   // The amount of valid elements in the buffer.
    capacity: u32, // The amount of capacity (in elements) that is available in the buffer.
    element: PhantomData<E>,
}

impl<E: Pod, const B: bool> GpuBuffer<E, B> {
    pub fn new(gl: &Gl) -> Self {
        Self {
            elements: gl.create_buffer().unwrap(),
            length: 0,
            capacity: 0,
            element: PhantomData,
        }
    }

    pub(crate) fn bind<'a>(&'a self, gl: &'a Gl) -> GpuBufferBinding<'a, E, B> {
        GpuBufferBinding::new(gl, self)
    }

    pub(crate) fn len(&self) -> u32 {
        self.length
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Only used once for an optimization in instance.rs.
    pub(crate) fn _elements(&self) -> &WebGlBuffer {
        &self.elements
    }

    pub(crate) fn buffer(&mut self, gl: &Gl, elements: &[E]) {
        self.length = elements.len().try_into().unwrap();

        // This can easily mess up the bind_buffer calls.
        debug_assert!(gl
            .get_parameter(OesVAO::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        // Make sure element's binding was cleared.
        debug_assert!(gl
            .get_parameter(GpuBufferType::fr(B).parameter())
            .unwrap()
            .is_null());

        // Don't bind if empty (length set earlier).
        if !self.is_empty() {
            // Buffer elements.
            let target = GpuBufferType::fr(B).target();
            gl.bind_buffer(target, Some(&self.elements));

            // Allocate buffer to nearest power of 2 (never shrinks).
            let new_cap = elements.len().next_power_of_two().try_into().unwrap();
            if new_cap > self.capacity {
                gl.buffer_data_with_i32(
                    target,
                    (new_cap * size_of::<E>() as u32) as i32,
                    Gl::DYNAMIC_DRAW,
                );
                self.capacity = new_cap;
            }

            let b = |a| gl.buffer_sub_data_with_i32_and_array_buffer_view(target, 0, a);

            unsafe {
                match GpuBufferType::fr(B) {
                    GpuBufferType::Array => {
                        b(&js_sys::Float32Array::view(bytemuck::cast_slice(elements)))
                    }
                    GpuBufferType::Element => match std::mem::size_of::<E>() {
                        1 => b(&js_sys::Uint8Array::view(bytemuck::cast_slice(elements))),
                        2 => b(&js_sys::Uint16Array::view(bytemuck::cast_slice(elements))),
                        4 => b(&js_sys::Uint32Array::view(bytemuck::cast_slice(elements))),
                        _ => panic!("invalid index size"),
                    },
                }
            }

            // Unbind (not required in release mode).
            #[cfg(debug_assertions)]
            gl.bind_buffer(target, None);
        }
    }
}

pub(crate) struct GpuBufferBinding<'a, E: Pod, const B: bool> {
    gl: &'a Gl,
    element: PhantomData<E>,
}

impl<'a, E: Pod, const B: bool> GpuBufferBinding<'a, E, B> {
    fn new(gl: &'a Gl, buffer: &GpuBuffer<E, B>) -> Self {
        // Make sure buffer element's binding was cleared.
        debug_assert!(gl
            .get_parameter(GpuBufferType::fr(B).parameter())
            .unwrap()
            .is_null());

        // Bind buffer's elements.
        gl.bind_buffer(GpuBufferType::fr(B).target(), Some(&buffer.elements));

        Self {
            gl,
            element: PhantomData,
        }
    }
}

impl<'a, V: Vertex> GpuBufferBinding<'a, V, { GpuBufferType::Array.to() }> {
    pub(crate) fn bind_attribs(&self) -> Attribs<'a> {
        let mut attribs = Attribs::new::<V>(self.gl);
        V::bind_attribs(&mut attribs);
        attribs
    }

    pub(crate) fn bind_attribs_instanced(&self, aia: &Aia, previous: Attribs<'a>) {
        V::bind_attribs(&mut Attribs::new_instanced::<V>(self.gl, aia, previous));
    }
}

impl<'a, E: Pod, const B: bool> Drop for GpuBufferBinding<'a, E, B> {
    fn drop(&mut self) {
        // Unbind (not required in release mode).
        #[cfg(debug_assertions)]
        self.gl.bind_buffer(GpuBufferType::fr(B).target(), None);
    }
}

/// RenderBuffer facilitates buffering a mesh to the GPU.
/// TODO find a better name because it's too similar to WebGlRenderBuffer.
pub struct RenderBuffer<V: Vertex, I: Index = u16> {
    pub(crate) vertices: GpuBuffer<V, { GpuBufferType::Array.to() }>,
    pub(crate) indices: GpuBuffer<I, { GpuBufferType::Element.to() }>,
    vao: WebGlVertexArrayObject,
}

impl<V: Vertex, I: Index> RenderBuffer<V, I> {
    pub fn new(gl: &Gl, oes: &OesVAO) -> Self {
        let buffer = Self {
            vertices: GpuBuffer::new(gl),
            indices: GpuBuffer::new(gl),
            vao: oes.create_vertex_array_oes().unwrap(),
        };

        // Make sure VAO was unbound.
        debug_assert!(gl
            .get_parameter(OesVAO::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        oes.bind_vertex_array_oes(Some(&buffer.vao));

        // Bind array buffer.
        let array_binding = buffer.vertices.bind(gl);
        array_binding.bind_attribs();

        // Bind element buffer.
        let element_binding = buffer.indices.bind(gl);

        // Unbinding VAO is ALWAYS required (unlike all other render unbinds).
        oes.bind_vertex_array_oes(None);

        // Unbind both buffers.
        drop(array_binding);
        drop(element_binding);

        buffer
    }

    pub(crate) fn bind<'a>(&'a self, gl: &'a Gl, oes: &'a OesVAO) -> RenderBufferBinding<'a, V, I> {
        RenderBufferBinding::new(gl, oes, self)
    }

    /// Copies a whole mesh into the render buffer.
    /// The mesh must have indices.
    pub fn buffer_mesh(&mut self, gl: &Gl, mesh: &MeshBuffer<V, I>) {
        assert!(
            mesh.default_indices || !mesh.indices.is_empty(),
            "mesh has no indices"
        );
        self.buffer(gl, mesh.vertices.as_slice(), mesh.indices.as_slice());
    }

    /// Copies vertices and indices into the render buffer.
    /// If indices is empty it performs array based rendering.
    // TODO get primitive.
    pub fn buffer(&mut self, gl: &Gl, vertices: &[V], indices: &[I]) {
        assert!(!vertices.is_empty(), "buffering no vertices");
        self.vertices.buffer(gl, vertices);
        self.indices.buffer(gl, indices);
    }
}

pub struct RenderBufferBinding<'a, V: Vertex, I: Index> {
    gl: &'a Gl,
    oes_vao: &'a OesVAO,
    buffer: &'a RenderBuffer<V, I>,
}

impl<'a, V: Vertex, I: Index> RenderBufferBinding<'a, V, I> {
    fn new(gl: &'a Gl, oes_vao: &'a OesVAO, buffer: &'a RenderBuffer<V, I>) -> Self {
        // Make sure buffer was unbound.
        debug_assert!(gl
            .get_parameter(OesVAO::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        oes_vao.bind_vertex_array_oes(Some(&buffer.vao));
        Self {
            gl,
            oes_vao,
            buffer,
        }
    }

    pub fn draw(&self, primitive: u32) {
        if !self.buffer.indices.is_empty() {
            self.gl.draw_elements_with_i32(
                primitive,
                self.buffer.indices.len() as i32,
                I::gl_enum(),
                0,
            );
        } else if !self.buffer.vertices.is_empty() {
            self.gl
                .draw_arrays(primitive, 0, self.buffer.vertices.len() as i32)
        }
    }
}

impl<'a, V: Vertex, I: Index> Drop for RenderBufferBinding<'a, V, I> {
    fn drop(&mut self) {
        // Unbind ALWAYS required (unlike all other render unbinds).
        self.oes_vao.bind_vertex_array_oes(None);
    }
}
