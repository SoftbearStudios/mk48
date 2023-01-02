// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::buffer::*;
use crate::gl::*;
use crate::index::Index;
use crate::renderer::{Layer, Renderer};
use crate::vertex::Vertex;
use crate::{DefaultRender, RenderLayer, ShaderBinding};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use web_sys::{WebGlBuffer, WebGlVertexArrayObject};

/// Differentiates [`MeshBuilder`]s that are being instanced by an [`InstanceLayer`]. The
/// [`Ord`] impl specifies the draw order.
pub trait MeshId: Clone + Hash + Eq + Ord {}
impl<T: Clone + Hash + Eq + Ord> MeshId for T {}

enum InnerBuffers<V, I, M> {
    Mesh(MeshBuilder<V, I>),
    Render(TriangleBuffer<V, I>, InstanceBuffer<M>),
}

impl<V, I, M> InnerBuffers<V, I, M> {
    fn unwrap_render(&self) -> (&TriangleBuffer<V, I>, &InstanceBuffer<M>) {
        if let InnerBuffers::Render(r, i) = self {
            (r, i)
        } else {
            unreachable!("pre_render was never called")
        }
    }
}

struct Buffers<V, I, M> {
    inner: InnerBuffers<V, I, M>,
    instances: Vec<M>,
}

impl<V: Vertex, I: Index, M: Vertex> Buffers<V, I, M> {
    fn pre_render(&mut self, renderer: &Renderer) {
        match &mut self.inner {
            InnerBuffers::Mesh(mesh) => {
                let mut render_buffer = TriangleBuffer::new(renderer);
                render_buffer.buffer_mesh(renderer, mesh);
                let instance_buffer = InstanceBuffer::new(renderer);

                self.inner = InnerBuffers::Render(render_buffer, instance_buffer);
                if let InnerBuffers::Render(_, instance_buffer) = &mut self.inner {
                    instance_buffer
                } else {
                    unreachable!()
                }
            }
            InnerBuffers::Render(_, instance_buffer) => instance_buffer,
        }
        .buffer(renderer, &self.instances);
        self.instances.clear();
    }
}

/// [`InstanceLayer`] facilitates drawing multiple [`MeshBuilder`]s multiple times.
pub struct InstanceLayer<V, I, M, ID> {
    buffers: HashMap<ID, Buffers<V, I, M>>,
    sorted_ids: Vec<ID>,
}

impl<V, I, M, ID> DefaultRender for InstanceLayer<V, I, M, ID> {
    fn new(_: &Renderer) -> Self {
        Self {
            buffers: Default::default(),
            sorted_ids: Default::default(),
        }
    }
}

impl<V, I, M, ID: MeshId> InstanceLayer<V, I, M, ID> {
    /// Draws an `instance` of the [`MeshBuilder`] previously created with the same `id` or calls
    /// `create`.
    pub fn draw(&mut self, id: ID, instance: M, create: impl FnOnce() -> MeshBuilder<V, I>) {
        self.buffers
            .entry(id.clone())
            .or_insert_with(|| {
                // Binary search will always return an error because self.buffers ensures id uniqueness.
                let index = self.sorted_ids.binary_search(&id).unwrap_err();

                // Creating sorted_ids this way is an O(n^2) operation where n is the number of ids
                // ever added, however for resonable ammount of ids (< 1000) it probably doesn't matter.
                // An alternative would be a BTreeSet which would be slower to iterate in render.
                self.sorted_ids.insert(index, id);

                Buffers {
                    inner: InnerBuffers::Mesh(create()),
                    instances: Default::default(),
                }
            })
            .instances
            .push(instance);
    }
}

impl<V: Vertex, I: Index, M: Vertex, ID: MeshId> Layer for InstanceLayer<V, I, M, ID> {
    fn pre_render(&mut self, renderer: &Renderer) {
        for buffers in self.buffers.values_mut() {
            buffers.pre_render(renderer);
        }
    }
}

impl<V: Vertex, I: Index, M: Vertex, ID: MeshId> RenderLayer<&ShaderBinding<'_>>
    for InstanceLayer<V, I, M, ID>
{
    fn render(&mut self, renderer: &Renderer, _: &ShaderBinding) {
        // Could store instances in sorted order if we had a fast way to index them.
        for b in self.sorted_ids.iter().map(|k| &self.buffers[k]) {
            // unwrap_render won't panic because pre_render ensures that all inners are render.
            let (render_buffer, instance_buffer) = b.inner.unwrap_render();
            if instance_buffer.is_empty() {
                continue; // Skip empty.
            }

            instance_buffer.bind(renderer, render_buffer).draw();
        }
    }
}

/// [`InstanceBuffer`] facilitates drawing a [`TriangleBuffer`] multiple times.
pub struct InstanceBuffer<M> {
    instances: GpuBuffer<M, { GpuBufferType::Array.to() }>,
    vao: WebGlVertexArrayObject,
    last_vertex_buffer: RefCell<Option<WebGlBuffer>>,
}

impl<M: Vertex> InstanceBuffer<M> {
    /// Creates a new [`InstanceBuffer`].
    pub fn new(renderer: &Renderer) -> Self {
        Self {
            instances: GpuBuffer::new(&renderer.gl),
            vao: renderer.ovao.create_vertex_array_oes().unwrap(),
            last_vertex_buffer: Default::default(),
        }
    }

    /// Returns true if there are no instances to draw (note does not check triangles).
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Binds the [`InstanceBuffer`] and a [`TriangleBuffer`] to draw instances of triangles.
    #[must_use]
    pub fn bind<'a, V: Vertex, I: Index>(
        &'a self,
        renderer: &'a Renderer,
        triangle_buffer: &'a TriangleBuffer<V, I>,
    ) -> InstanceBufferBinding<'a, V, I, M> {
        let gl = &renderer.gl;
        let aia = renderer
            .aia
            .as_ref()
            .expect("must enable AngleInstancedArrays");
        let ovao = &renderer.ovao;

        // Don't redo attribs if buffer doesn't change.
        let mut last_vertex_buffer = self.last_vertex_buffer.borrow_mut();
        let vertex_buffer = triangle_buffer.vertices._elements();
        if last_vertex_buffer.as_ref() != Some(vertex_buffer) {
            *last_vertex_buffer = Some(vertex_buffer.clone());

            // Make sure VAO was unbound.
            debug_assert!(gl
                .get_parameter(Ovao::VERTEX_ARRAY_BINDING_OES)
                .unwrap()
                .is_null());

            ovao.bind_vertex_array_oes(Some(&self.vao));

            // Bind array buffer.
            let array_binding = triangle_buffer.vertices.bind(gl);
            let attribs = array_binding.bind_attribs();

            // Unbind early so can bind instance buffer.
            drop(array_binding);

            // Bind element buffer.
            let element_binding = triangle_buffer.indices.bind(gl);

            // Bind instance buffer.
            let instance_binding = self.instances.bind(gl);
            instance_binding.bind_attribs_instanced(aia, attribs);

            // Unbinding VAO is ALWAYS required (unlike all other render unbinds).
            ovao.bind_vertex_array_oes(None);

            // Unbind all other buffers.
            drop(element_binding);
            drop(instance_binding);
        }

        InstanceBufferBinding::new(gl, aia, ovao, self, triangle_buffer)
    }

    /// Copies instances into the [`InstanceBuffer`].
    pub fn buffer(&mut self, renderer: &Renderer, instances: &[M]) {
        self.instances.buffer(&renderer.gl, instances);
    }
}

/// A bound [`InstanceBuffer`] that can draw instances of triangles.
pub struct InstanceBufferBinding<'a, V, I, M> {
    aia: &'a Aia,
    ovao: &'a Ovao,
    triangle_buffer: &'a TriangleBuffer<V, I>,
    buffer: &'a InstanceBuffer<M>,
}

impl<'a, V: Vertex, I: Index, M: Vertex> InstanceBufferBinding<'a, V, I, M> {
    fn new(
        gl: &'a Gl,
        aia: &'a Aia,
        ovao: &'a Ovao,
        buffer: &'a InstanceBuffer<M>,
        triangle_buffer: &'a TriangleBuffer<V, I>,
    ) -> Self {
        // Make sure buffer was unbound.
        debug_assert!(gl
            .get_parameter(Ovao::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        ovao.bind_vertex_array_oes(Some(&buffer.vao));

        Self {
            aia,
            ovao,
            triangle_buffer,
            buffer,
        }
    }

    /// Draws instances of triangles.
    pub fn draw(&self) {
        let primitive = Gl::TRIANGLES;
        if !self.triangle_buffer.indices.is_empty() {
            self.aia.draw_elements_instanced_angle_with_i32(
                primitive,
                self.triangle_buffer.indices.len() as i32,
                I::GL_ENUM,
                0,
                self.buffer.instances.len() as i32,
            );
        } else if !self.triangle_buffer.vertices.is_empty() {
            self.aia.draw_arrays_instanced_angle(
                primitive,
                0,
                self.triangle_buffer.vertices.len() as i32,
                self.buffer.instances.len() as i32,
            )
        }
    }
}

impl<'a, V, I, M> Drop for InstanceBufferBinding<'a, V, I, M> {
    fn drop(&mut self) {
        // Unbind ALWAYS required (unlike all other render unbinds).
        self.ovao.bind_vertex_array_oes(None);
    }
}
