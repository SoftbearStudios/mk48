// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::buffer::{RenderBuffer, RenderBufferBinding};
use crate::renderer::shader::{Shader, ShaderBinding};
use crate::renderer::texture::{Texture, TextureBinding, TextureFormat};
use crate::renderer::vertex::{PosUv, Vertex};
use glam::{uvec2, UVec2, Vec2, Vec4};
use serde::Serialize;
use std::cell::Cell;
use std::mem;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, OesElementIndexUint, OesStandardDerivatives, OesVertexArrayObject,
    WebGlRenderingContext as Gl,
};

/// For compiling shaders in parallel.
pub(crate) struct KhrParallelShaderCompile;
impl KhrParallelShaderCompile {
    pub const COMPLETION_STATUS_KHR: u32 = 37297;
}

/// Anything in the rendering pipeline. Can be derived on a struct which has fields that also
/// implement it.
pub trait Layer {
    /// Called before game gets to queue rendering.
    fn pre_prepare(&mut self, _: &Renderer) {}

    /// Called before rendering.
    /// Useful for buffering textures etc.
    fn pre_render(&mut self, _: &Renderer) {}

    /// Renders the layer.
    fn render(&mut self, renderer: &Renderer);
}

use crate::renderer::camera::Camera;
use crate::renderer::index::Index;
pub use engine_macros::Layer;

/// A general WebGL renderer, focused on 2d for now.
pub struct Renderer {
    /// HTML Canvas.
    canvas: HtmlCanvasElement,
    cached_canvas_size: Cell<Option<UVec2>>,
    /// WebGL context.
    pub(crate) gl: Gl,
    /// WebGL extensions.
    pub(crate) khr: Option<KhrParallelShaderCompile>,
    pub(crate) oes_vao: OesVertexArrayObject,
    /// Camera information.
    pub camera: Camera,
    pub aligned_camera: Camera,
    /// Timing information
    pub time: f32,
    pub time_delta: f32,
    /// Caches.
    pub(crate) background_buffer: Option<RenderBuffer<PosUv>>,
    pub(crate) text_shader: Option<Shader>,
    pub(crate) graphic_shader: Option<Shader>,
    pub(crate) particle_shader: Option<Shader>,
    pub(crate) sprite_shader: Option<Shader>,
}

impl Renderer {
    // Creates a new WebGl 1.0 render, attaching it to the canvas element with the id "canvas."
    pub fn new(antialias: bool) -> Self {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement =
            canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct ContextOptions {
            alpha: bool,
            antialias: bool,
            power_preference: &'static str,
            premultiplied_alpha: bool,
            preserve_drawing_buffer: bool,
        }

        let options = serde_wasm_bindgen::to_value(&ContextOptions {
            alpha: true,
            antialias,
            power_preference: "high-performance",
            premultiplied_alpha: true,
            preserve_drawing_buffer: false,
        })
        .unwrap();

        let gl = canvas
            .get_context_with_context_options("webgl", &options)
            .unwrap()
            .expect("could not create webgl context")
            .dyn_into::<Gl>()
            .unwrap();

        let khr = gl
            .get_extension("KHR_parallel_shader_compile")
            .unwrap()
            .map(|_| KhrParallelShaderCompile);

        let oes_vao = gl
            .get_extension("OES_vertex_array_object")
            .unwrap()
            .unwrap()
            .unchecked_into::<OesVertexArrayObject>();

        gl.enable(Gl::BLEND);

        // First argument is Gl::SRC_ALPHA if not premultiplied alpha, Gl::ONE if premultiplied(?).
        gl.blend_func(Gl::ONE, Gl::ONE_MINUS_SRC_ALPHA);

        Self {
            canvas,
            cached_canvas_size: Cell::new(None),
            gl,
            khr,
            oes_vao,
            camera: Camera::new(false),
            aligned_camera: Camera::new(true),
            time: 0.0,
            background_buffer: None,
            text_shader: None,
            graphic_shader: None,
            particle_shader: None,
            sprite_shader: None,
            time_delta: 0.0,
        }
    }

    /// Returns if highp is supported in a fragment shader.
    pub fn fragment_has_highp(&self) -> bool {
        let precison = self
            .gl
            .get_shader_precision_format(Gl::FRAGMENT_SHADER, Gl::HIGH_FLOAT)
            .unwrap();
        precison.precision() >= 23
    }

    /// Returns mediump is not just an alias for highp in a fragment shader.
    pub fn fragment_uses_mediump(&self) -> bool {
        let precison = self
            .gl
            .get_shader_precision_format(Gl::FRAGMENT_SHADER, Gl::MEDIUM_FLOAT)
            .unwrap();
        precison.precision() < 23
    }

    /// Call early on if any custom shaders need OES standard derivatives.
    pub fn enable_oes_standard_derivatives(&self) {
        let oes_standard_derivatives = self
            .gl
            .get_extension("OES_standard_derivatives")
            .unwrap()
            .unwrap()
            .unchecked_into::<OesStandardDerivatives>();

        // No need to access this from Rust later.
        mem::forget(oes_standard_derivatives);
    }

    /// Allow using u32 as index in WebGl1.
    pub fn enable_oes_element_index_uint(&self) {
        let oes_element_index_uint = self
            .gl
            .get_extension("OES_element_index_uint")
            .unwrap()
            .unwrap()
            .unchecked_into::<OesElementIndexUint>();

        mem::forget(oes_element_index_uint);
    }

    /// Creates a new shader from vertex and fragment GLSL.
    pub fn create_shader(&self, vertex_source: &str, fragment_source: &str) -> Shader {
        Shader::new(&self.gl, vertex_source, fragment_source)
    }

    pub fn canvas_size(&self) -> UVec2 {
        let cached_size = self.cached_canvas_size.get();
        if let Some(size) = cached_size {
            size
        } else {
            let size = uvec2(self.canvas.width(), self.canvas.height());
            self.cached_canvas_size.set(Some(size));
            size
        }
    }

    /// Returns the aspect ratio (width/height) of the canvas.
    pub fn aspect_ratio(&self) -> f32 {
        self.camera.aspect_ratio()
    }

    /// color is RGBA with components 0.0-1.0.
    /// Will take effect next frame.
    pub fn set_background_color(&mut self, color: Vec4) {
        self.gl.clear_color(color.x, color.y, color.z, color.w);
    }

    pub(crate) fn pre_prepare(&mut self, layer: &mut impl Layer) {
        self.cached_canvas_size.set(None);
        layer.pre_prepare(self);
    }

    /// start starts the renderer changing the aspect ratio if necessary, clearing the screen.
    pub(crate) fn render(&mut self, layer: &mut impl Layer, time_seconds: f32) {
        // Reset caches.
        self.time_delta = time_seconds - self.time;
        self.time = time_seconds;

        // Pre-render such as allocating textures.
        layer.pre_render(self);

        // Set viewport and clear webgl.
        let size = self.canvas_size().as_ivec2();
        self.gl.viewport(0, 0, size.x, size.y);
        self.gl.clear(Gl::COLOR_BUFFER_BIT);

        // Render everything.
        layer.render(self);
    }

    /// Gets the camera center.
    pub fn camera_center(&self) -> Vec2 {
        self.camera.center()
    }

    /// Defines the view matrix.
    /// MUST be called before any layers are updated.
    pub fn set_camera(&mut self, center: Vec2, zoom: f32) {
        let viewport = self.canvas_size();
        self.camera.update(center, zoom, viewport);
        self.aligned_camera.update(center, zoom, viewport);
    }

    /// Convert a position in view space (-1..1) to world space.
    pub fn to_world_position(&self, view_position: Vec2) -> Vec2 {
        self.camera.camera_matrix.transform_point2(view_position)
    }

    /// Lower level function to bind a buffer.
    pub fn bind_buffer<'a, V: Vertex, I: Index>(
        &'a self,
        buffer: &'a RenderBuffer<V, I>,
    ) -> RenderBufferBinding<'a, V, I> {
        buffer.bind(&self.gl, &self.oes_vao)
    }

    /// Lower level function to bind a shader. Can return None if the shader isn't compiled yet.
    pub fn bind_shader<'a>(&'a self, shader: &'a Shader) -> Option<ShaderBinding<'a>> {
        shader.bind(&self.gl, self.khr.as_ref())
    }

    /// Lower level function to bind a texture.
    pub fn bind_texture(&self, texture: &Texture, index: usize) -> TextureBinding {
        texture.bind(&self.gl, index)
    }

    /// Loads an RBGA texture from a URL.
    /// TODO remove dimensions from this function.
    pub fn load_texture(
        &self,
        img_src: &str,
        dimensions: UVec2,
        placeholder: Option<[u8; 3]>,
        repeating: bool,
    ) -> Texture {
        Texture::load(&self.gl, img_src, dimensions, placeholder, repeating)
    }

    /// Creates a new empty texture with the given formatting and fitler.
    /// Mipmaps and repeating cannot be used.
    pub fn new_empty_texture(&self, format: TextureFormat, linear_filter: bool) -> Texture {
        Texture::new_empty(&self.gl, format, linear_filter)
    }

    /// Copies the bytes to the texture, resizing it if necessary.
    pub fn realloc_texture_with_opt_bytes(
        &self,
        texture: &mut Texture,
        dimensions: UVec2,
        bytes: Option<&[u8]>,
    ) {
        texture.realloc_with_opt_bytes(&self.gl, dimensions, bytes);
    }
}
