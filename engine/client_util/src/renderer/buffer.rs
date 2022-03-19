// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::attribute::Attribs;
use crate::renderer::index::Index;
use crate::renderer::vertex::Vertex;
use std::marker::PhantomData;
use std::mem::size_of;
use std::slice;
use web_sys::{
    OesVertexArrayObject as OesVAO, WebGlBuffer, WebGlRenderingContext as Gl,
    WebGlVertexArrayObject,
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

/// RenderBuffer facilitates buffering a mesh to the GPU.
/// TODO find a better name because it's too similar to WebGlRenderBuffer.
pub struct RenderBuffer<V: Vertex, I: Index = u16> {
    vertices: WebGlBuffer,
    vertices_capacity: usize, // The amount of capacity in vertices that is available in the buffer.
    indices: WebGlBuffer,
    indices_capacity: usize, // The amount of capacity in indices that is available in the buffer.
    vao: WebGlVertexArrayObject,
    index_count: u32,
    vertex_count: u32,
    vertex: PhantomData<V>,
    index: PhantomData<I>,
}

impl<V: Vertex, I: Index> RenderBuffer<V, I> {
    pub fn new(gl: &Gl, oes: &OesVAO) -> Self {
        let buffer = Self {
            vertices: gl.create_buffer().unwrap(),
            vertices_capacity: 0,
            indices: gl.create_buffer().unwrap(),
            indices_capacity: 0,
            vao: oes.create_vertex_array_oes().unwrap(),
            index_count: 0,
            vertex_count: 0,
            vertex: PhantomData,
            index: PhantomData,
        };

        // Make sure array was unbound.
        debug_assert!(gl
            .get_parameter(OesVAO::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        // Make sure both bindings were cleared.
        debug_assert!(gl
            .get_parameter(Gl::ARRAY_BUFFER_BINDING)
            .unwrap()
            .is_null());
        debug_assert!(gl
            .get_parameter(Gl::ELEMENT_ARRAY_BUFFER_BINDING)
            .unwrap()
            .is_null());

        oes.bind_vertex_array_oes(Some(&buffer.vao));

        // Bind buffers to vao.
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&buffer.vertices));
        V::bind_attribs(&mut Attribs::new(gl));
        gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(&buffer.indices));

        // Unbind ALWAYS required (unlike all other render unbinds).
        oes.bind_vertex_array_oes(None);

        // Unbind both buffers (not required in release mode).
        #[cfg(debug_assertions)]
        {
            gl.bind_buffer(Gl::ARRAY_BUFFER, None);
            gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, None);
        }

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
    pub fn buffer(&mut self, gl: &Gl, vertices: &[V], indices: &[I]) {
        assert!(!vertices.is_empty(), "buffering no vertices");

        self.index_count = indices.len() as u32;
        self.vertex_count = vertices.len() as u32;

        // This can easily mess up the bind_buffer calls.
        debug_assert!(gl
            .get_parameter(OesVAO::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        // Make sure vertex binding was cleared.
        debug_assert!(gl
            .get_parameter(Gl::ARRAY_BUFFER_BINDING)
            .unwrap()
            .is_null());

        // Buffer vertices.
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.vertices));

        // Allocate buffer to nearest power of 2 (never shrinks).
        let new_cap = vertices.len().next_power_of_two();
        if new_cap > self.vertices_capacity {
            gl.buffer_data_with_i32(
                Gl::ARRAY_BUFFER,
                (new_cap * size_of::<V>()) as i32,
                Gl::DYNAMIC_DRAW,
            );
            self.vertices_capacity = new_cap;
        }

        if self.vertex_count > 0 {
            unsafe {
                // Points to raw rust memory so can't allocate while in use.
                let vert_array = js_sys::Float32Array::view(floats_from_vertices(vertices));
                gl.buffer_sub_data_with_i32_and_array_buffer_view(Gl::ARRAY_BUFFER, 0, &vert_array);
            }
        }

        // Unbind (not required in release mode).
        #[cfg(debug_assertions)]
        gl.bind_buffer(Gl::ARRAY_BUFFER, None);

        // Make sure index binding was cleared.
        debug_assert!(gl
            .get_parameter(Gl::ELEMENT_ARRAY_BUFFER_BINDING)
            .unwrap()
            .is_null());

        // Buffer indices.
        gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(&self.indices));

        // Allocate buffer to nearest power of 2 (never shrinks).
        let new_cap = indices.len().next_power_of_two();
        if new_cap > self.indices_capacity {
            gl.buffer_data_with_i32(
                Gl::ELEMENT_ARRAY_BUFFER,
                (new_cap * size_of::<I>()) as i32,
                Gl::DYNAMIC_DRAW,
            );
            self.indices_capacity = new_cap;
        }

        if self.index_count > 0 {
            unsafe {
                // Points to raw rust memory so can't allocate while in use.
                let elem_array = I::view(indices);
                gl.buffer_sub_data_with_i32_and_array_buffer_view(
                    Gl::ELEMENT_ARRAY_BUFFER,
                    0,
                    &elem_array,
                );
            }
        }

        // Unbind (not required in release mode).
        #[cfg(debug_assertions)]
        gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, None);
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
        if self.buffer.index_count != 0 {
            self.gl.draw_elements_with_i32(
                primitive,
                self.buffer.index_count as i32,
                I::gl_enum(),
                0,
            );
        } else if self.buffer.vertex_count != 0 {
            self.gl
                .draw_arrays(primitive, 0, self.buffer.vertex_count as i32)
        }
    }
}

impl<'a, V: Vertex, I: Index> Drop for RenderBufferBinding<'a, V, I> {
    fn drop(&mut self) {
        // Unbind ALWAYS required (unlike all other render unbinds).
        self.oes_vao.bind_vertex_array_oes(None);
    }
}

/// Reinterprets a slice of floats as a slice of vertices, panicking if the
/// given number of floats is not evenly divided by the vertex size.
#[allow(unused)]
pub fn vertices_from_floats<V: Vertex>(floats: &[f32]) -> &[V] {
    assert_eq!(floats.len() % V::floats(), 0);

    unsafe {
        let ptr = &floats[0] as *const f32 as *const V;
        let len = floats.len() / V::floats();
        slice::from_raw_parts(ptr, len)
    }
}

/// Opposite of vertices_from_floats.
pub fn floats_from_vertices<V: Vertex>(vertices: &[V]) -> &[f32] {
    unsafe {
        let ptr = &vertices[0] as *const V as *const f32;
        let len = vertices.len() * V::floats();
        slice::from_raw_parts(ptr, len)
    }
}
