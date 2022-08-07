use crate::renderer::buffer::*;
use crate::renderer::index::Index;
use crate::renderer::renderer::{Layer, Renderer};
use crate::renderer::shader::Shader;
use crate::renderer::vertex::Vertex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::mem::take;
use web_sys::{
    AngleInstancedArrays as Aia, OesVertexArrayObject as OesVAO, WebGlBuffer,
    WebGlRenderingContext as Gl, WebGlVertexArrayObject,
};

pub trait MeshId: Clone + Hash + Eq + Ord {}
impl<T: Clone + Hash + Eq + Ord> MeshId for T {}

struct Buffers<V: Vertex, I: Index, M: Vertex> {
    render_buffer: RenderBuffer<V, I>,
    instance_buffer: InstanceBuffer<M>,
}

impl<V: Vertex, I: Index, M: Vertex> Buffers<V, I, M> {
    fn new(gl: &Gl, oes: &OesVAO) -> Self {
        Self {
            render_buffer: RenderBuffer::new(gl, oes),
            instance_buffer: InstanceBuffer::new(gl, oes),
        }
    }
}

pub struct InstanceLayer<V: Vertex, I: Index, M: Vertex, ID: MeshId> {
    shader: Shader,
    buffers: HashMap<ID, Buffers<V, I, M>>,
    mesh_buffers: HashMap<ID, MeshBuffer<V, I>>,
    instances: HashMap<ID, Vec<M>>,
}

impl<V: Vertex, I: Index, M: Vertex, ID: MeshId> InstanceLayer<V, I, M, ID> {
    pub fn new(renderer: &mut Renderer, vertex: &str, fragment: &str) -> Self {
        let gl = &renderer.gl;
        let shader = Shader::new(gl, vertex, fragment);

        Self {
            shader,
            buffers: Default::default(),
            mesh_buffers: Default::default(),
            instances: Default::default(),
        }
    }

    pub fn add_instance<F: FnMut() -> MeshBuffer<V, I>>(&mut self, id: ID, instance: M, create: F) {
        if self.buffers.get(&id).is_none() {
            self.mesh_buffers.entry(id.clone()).or_insert_with(create);
        }
        self.instances.entry(id).or_default().push(instance);
    }
}

impl<V: Vertex, I: Index, M: Vertex, ID: MeshId> Layer for InstanceLayer<V, I, M, ID> {
    fn pre_render(&mut self, renderer: &Renderer) {
        let gl = &renderer.gl;

        for (id, mesh) in take(&mut self.mesh_buffers) {
            let mut buffers = Buffers::new(gl, &renderer.oes_vao);
            buffers.render_buffer.buffer_mesh(gl, &mesh);
            self.buffers.insert(id, buffers);
        }

        for (id, instances) in &mut self.instances {
            let buffers = self.buffers.get_mut(id).unwrap();
            buffers.instance_buffer.buffer(gl, instances);
            instances.clear()
        }
    }

    fn render(&mut self, r: &Renderer) {
        let aia = r.aia.as_ref().expect("must enable AngleInstancedArrays");

        if let Some(binding) = r.bind_shader(&self.shader) {
            binding.uniform_matrix3f("uView", &r.camera.view_matrix);
            let mut keys: Vec<_> = self
                .buffers
                .iter()
                .filter_map(|(k, v)| (!v.instance_buffer.is_empty()).then_some(k)) // Skip empty.
                .cloned()
                .collect();

            keys.sort_unstable();

            // TODO could store values along side keys to skip looking them up.
            for b in keys.iter().map(|k| &self.buffers[k]) {
                b.instance_buffer
                    .bind(&r.gl, aia, &r.oes_vao, &b.render_buffer)
                    .draw(Gl::TRIANGLES);
            }
        }
    }
}

pub struct InstanceBuffer<M: Vertex> {
    instances: GpuBuffer<M, { GpuBufferType::Array.to() }>,
    vao: WebGlVertexArrayObject,
    last_vertex_buffer: RefCell<Option<WebGlBuffer>>,
}

impl<M: Vertex> InstanceBuffer<M> {
    pub fn new(gl: &Gl, oes: &OesVAO) -> Self {
        let buffer = Self {
            instances: GpuBuffer::new(gl),
            vao: oes.create_vertex_array_oes().unwrap(),
            last_vertex_buffer: Default::default(),
        };

        buffer
    }

    // TODO maybe return Option<InstanceBufferBinding> from bind instead of exposing this.
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub(crate) fn bind<'a, V: Vertex, I: Index>(
        &'a self,
        gl: &'a Gl,
        aia: &'a Aia,
        oes: &'a OesVAO,
        render_buffer: &'a RenderBuffer<V, I>,
    ) -> InstanceBufferBinding<'a, V, I, M> {
        // Don't redo attribs if buffer doesn't change.
        let mut last_vertex_buffer = self.last_vertex_buffer.borrow_mut();
        let vertex_buffer = render_buffer.vertices._elements();
        if last_vertex_buffer.as_ref() != Some(vertex_buffer) {
            *last_vertex_buffer = Some(vertex_buffer.clone());

            // Make sure VAO was unbound.
            debug_assert!(gl
                .get_parameter(OesVAO::VERTEX_ARRAY_BINDING_OES)
                .unwrap()
                .is_null());

            oes.bind_vertex_array_oes(Some(&self.vao));

            // Bind array buffer.
            let array_binding = render_buffer.vertices.bind(gl);
            let attribs = array_binding.bind_attribs();

            // Unbind early so can bind instance buffer.
            drop(array_binding);

            // Bind element buffer.
            let element_binding = render_buffer.indices.bind(gl);

            // Bind instance buffer.
            let instance_binding = self.instances.bind(gl);
            instance_binding.bind_attribs_instanced(aia, attribs);

            // Unbinding VAO is ALWAYS required (unlike all other render unbinds).
            oes.bind_vertex_array_oes(None);

            // Unbind all other buffers.
            drop(element_binding);
            drop(instance_binding);
        }

        InstanceBufferBinding::new(gl, aia, oes, self, render_buffer)
    }

    /// Copies instances into the instance buffer.
    pub fn buffer(&mut self, gl: &Gl, instances: &[M]) {
        self.instances.buffer(gl, instances);
    }
}

pub struct InstanceBufferBinding<'a, V: Vertex, I: Index, M: Vertex> {
    aia: &'a Aia,
    oes_vao: &'a OesVAO,
    render_buffer: &'a RenderBuffer<V, I>,
    buffer: &'a InstanceBuffer<M>,
}

impl<'a, V: Vertex, I: Index, M: Vertex> InstanceBufferBinding<'a, V, I, M> {
    fn new(
        gl: &'a Gl,
        aia: &'a Aia,
        oes_vao: &'a OesVAO,
        buffer: &'a InstanceBuffer<M>,
        render_buffer: &'a RenderBuffer<V, I>,
    ) -> Self {
        // Make sure buffer was unbound.
        debug_assert!(gl
            .get_parameter(OesVAO::VERTEX_ARRAY_BINDING_OES)
            .unwrap()
            .is_null());

        oes_vao.bind_vertex_array_oes(Some(&buffer.vao));

        Self {
            aia,
            oes_vao,
            render_buffer,
            buffer,
        }
    }

    // TODO get primitive in RenderBuffer::buffer.
    pub fn draw(&self, primitive: u32) {
        if !self.render_buffer.indices.is_empty() {
            self.aia.draw_elements_instanced_angle_with_i32(
                primitive,
                self.render_buffer.indices.len() as i32,
                I::gl_enum(),
                0,
                self.buffer.instances.len() as i32,
            );
        } else if !self.render_buffer.vertices.is_empty() {
            self.aia.draw_arrays_instanced_angle(
                primitive,
                0,
                self.render_buffer.vertices.len() as i32,
                self.buffer.instances.len() as i32,
            )
        }
    }
}

impl<'a, V: Vertex, I: Index, M: Vertex> Drop for InstanceBufferBinding<'a, V, I, M> {
    fn drop(&mut self) {
        // Unbind ALWAYS required (unlike all other render unbinds).
        self.oes_vao.bind_vertex_array_oes(None);
    }
}
