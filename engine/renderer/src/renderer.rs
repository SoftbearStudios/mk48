// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::camera::Camera;
use crate::gl::*;
use crate::shader::{Shader, ShaderBinding};
pub use engine_macros::Layer;
use glam::*;
use js_hooks::error_message;
use linear_map::LinearMap;
use std::cell::{Cell, RefCell};
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

/// Contains things that can be drawn. Can be derived on structs that have fields which also
/// implement it. If your struct has non [`Layer`] fields you can label the [`Layer`] fields with
/// `#[layer]`. You can also specify a camera bound with `#[layer(Camera2d)]`.
pub trait Layer<C> {
    // TODO for ease of use make DefaultLayer with this.
    // fn new(renderer: &Renderer<C>) -> Self;

    /// Called at the start of each frame. Useful for copying the [`Renderer`]'s time.
    fn pre_prepare(&mut self, renderer: &Renderer<C>) {
        let _ = renderer;
    }

    /// Called right before any [`Layer`] is [`render`][`Layer::render`]ed. Useful for creating textures.
    fn pre_render(&mut self, renderer: &Renderer<C>) {
        let _ = renderer;
    }

    /// Renders the [`Layer`].
    fn render(&mut self, renderer: &Renderer<C>);
}

/// Extends a [`Layer`] with a custom [`Shader`].
pub trait LayerShader<C> {
    /// Create a custom [`Shader`].
    fn create(&self, renderer: &Renderer<C>) -> Shader;
    /// Useful for setting any additional uniforms.
    fn prepare(&mut self, renderer: &Renderer<C>, shader: &ShaderBinding) {
        let _ = (renderer, shader);
    }
}

/// Macro to easily define a [`LayerShader`] that only overrides [`LayerShader::create`].
#[macro_export]
macro_rules! layer_shader {
    ($name:ident,$camera:ident,$vert:literal,$frag:literal) => {
        #[derive(Default)]
        pub struct $name;
        impl $crate::LayerShader<$camera> for $name {
            fn create(&self, renderer: &$crate::Renderer<$camera>) -> $crate::Shader {
                renderer.create_shader(include_str!($vert), include_str!($frag))
            }
        }
    };
}

/// An abstraction over
/// [WebGL](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.WebGlRenderingContext.html)/
/// [WebGL2](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.WebGl2RenderingContext.html)
/// that can be used in 2D and 3D applications.
pub struct Renderer<C> {
    /// HTML Canvas.
    canvas: HtmlCanvasElement,
    cached_canvas_size: Cell<Option<UVec2>>,
    /// WebGL context.
    pub(crate) gl: Gl,
    /// WebGL extensions.
    pub(crate) aia: Option<Aia>,
    pub(crate) khr: Option<Khr>,
    pub(crate) ovao: Ovao,
    /// The [`Camera`].
    pub camera: C,
    /// Current time in seconds since start.
    pub time: f32,
    /// Seconds since last frame.
    pub time_delta: f32,
    /// Cache of static shaders.
    shader_cache: RefCell<LinearMap<(&'static str, &'static str), Shader>>,
    /// WebGL doesn't support antialiasing with srgb.
    #[cfg(feature = "srgb")]
    pub(crate) antialiasing: Option<i32>,
    /// How much anisotropy to use or None if shouldn't use.
    #[cfg(feature = "anisotropy")]
    pub(crate) anisotropy: Option<u32>,
}

impl<C: Camera> Renderer<C> {
    // Creates a new WebGL/WebGL2 render, attaching it to the canvas element with the id "canvas."
    #[doc(hidden)]
    pub fn new(antialias: bool) -> Result<Self, String> {
        let builtin_antialiasing = antialias && !cfg!(feature = "srgb");

        let canvas = js_hooks::canvas();
        let options = js_sys::JSON::parse(&format!(
            r##"{{
            "alpha": true,
            "antialias": {builtin_antialiasing},
            "power_preference": "high-performance",
            "premultiplied_alpha": true,
            "preserve_drawing_buffer": false
        }}"##
        ))
        .unwrap();

        // See: https://developer.mozilla.org/en-US/docs/Web/API/HTMLCanvasElement/getContext
        let gl = canvas
            .get_context_with_context_options(GL_NAME, &options)
            .map_err(|e| {
                error_message(&e)
                    .unwrap_or_else(|| concat!("Error initializing ", gl_title!()).into())
            })?
            .ok_or_else(|| concat!(gl_title!(), " unsupported"))?
            .dyn_into::<Gl>()
            .unwrap();

        crate::texture::reset_active_texture();

        let khr = gl
            .get_extension("KHR_parallel_shader_compile")
            .unwrap()
            .map(|_| KhrParallelShaderCompile);

        let ovao = gl.get_extension_ovao();

        // WebGL2 has this built in by default. In WebGL we only need to enable it, not save it.
        #[cfg(all(not(feature = "webgl2"), feature = "srgb"))]
        gl.get_extension("EXT_sRGB").unwrap().unwrap();

        // Must perform antialiasing in render buffer in using srgb and webgl2.
        #[cfg(feature = "srgb")]
        let antialiasing = antialias.then(|| {
            // Use MSAA.
            #[cfg(feature = "webgl2")]
            return gl.get_parameter(Gl::MAX_SAMPLES).unwrap().as_f64().unwrap() as i32;
            // Use FXAA.
            #[cfg(not(feature = "webgl2"))]
            -1
        });

        gl.enable(Gl::BLEND);

        // First argument is Gl::SRC_ALPHA if not premultiplied alpha, Gl::ONE if premultiplied(?).
        gl.blend_func(Gl::ONE, Gl::ONE_MINUS_SRC_ALPHA);

        let res = Ok(Self {
            canvas,
            cached_canvas_size: Cell::new(None),
            gl,
            aia: None,
            khr,
            ovao,
            camera: Default::default(),
            time: 0.0,
            time_delta: 0.0,
            shader_cache: Default::default(),
            #[cfg(feature = "srgb")]
            antialiasing,
            #[cfg(feature = "anisotropy")]
            anisotropy: None,
        });
        C::init_render(res.as_ref().unwrap());
        res
    }
}

impl<C> Renderer<C> {
    /// Returns if highp is supported in a fragment shader.
    #[deprecated = "should assume fragment has highp"]
    pub fn fragment_has_highp(&self) -> bool {
        let precison = self
            .gl
            .get_shader_precision_format(Gl::FRAGMENT_SHADER, Gl::HIGH_FLOAT)
            .unwrap();
        precison.precision() >= 23
    }

    /// Returns mediump is not just an alias for highp in a fragment shader.
    #[deprecated = "not very accurate, prefer good defaults instead"]
    pub fn fragment_uses_mediump(&self) -> bool {
        let precison = self
            .gl
            .get_shader_precision_format(Gl::FRAGMENT_SHADER, Gl::MEDIUM_FLOAT)
            .unwrap();
        precison.precision() < 23
    }

    /// Call early on if using instancing. Still required if using WebGL2.
    pub fn enable_angle_instanced_arrays(&mut self) {
        self.aia = Some(self.gl.get_extension_aia());
    }

    /// Call early on if any custom shaders need OES standard derivatives.
    /// Only available if not using WebGL2, since shaders with
    /// `#extension GL_OES_standard_derivatives : enable` won't compile in WebGL2.
    /// If you want the same functionality you must use `#version 300 es`.
    #[cfg(not(feature = "webgl2"))]
    pub fn enable_oes_standard_derivatives(&self) {
        // We only need to enable it, not save it.
        self.gl
            .get_extension("OES_standard_derivatives")
            .unwrap()
            .unwrap();
    }

    /// Call early on if using [`prim@u32`] as [`Index`][`crate::index::Index`].
    pub fn enable_oes_element_index_uint(&self) {
        // WebGL2 has this built in by default. In WebGL we only need to enable it, not save it.
        #[cfg(not(feature = "webgl2"))]
        self.gl
            .get_extension("OES_element_index_uint")
            .unwrap()
            .unwrap();
    }

    /// Call early on if you want textures to be sampled with anisotropy. `anisotropy_limit` limits
    /// the hardware anisotropy available for performance concerns.
    #[cfg(feature = "anisotropy")]
    pub fn set_anisotropy_limit(&mut self, anisotropy_limit: u32) {
        // We only need to enable it, not save it.
        let ext = self.gl.get_extension("EXT_texture_filter_anisotropic");

        if ext.map_or(false, |v| v.is_some()) {
            self.anisotropy = Some(
                (self
                    .gl
                    .get_parameter(Ani::MAX_TEXTURE_MAX_ANISOTROPY_EXT)
                    .unwrap()
                    .as_f64()
                    .unwrap() as u32)
                    .min(anisotropy_limit),
            );
        }
    }

    /// Returns the aspect ratio (width / height) of the canvas.
    pub fn aspect_ratio(&self) -> f32 {
        viewport_to_aspect(self.canvas_size())
    }

    /// Size of the canvas in real pixels (doesn't account for device pixel ratio).
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

    /// Creates a new shader from static glsl sources. Only compiles each shader once.
    /// For runtime defined shaders use [`Shader::new`].
    pub fn create_shader(&self, vertex: &'static str, fragment: &'static str) -> Shader {
        self.shader_cache
            .borrow_mut()
            .entry((vertex, fragment))
            .or_insert_with(|| Shader::new(self, vertex, fragment))
            .clone()
    }

    /// Enables the depth test with depth func less. Note all 2D [`Layer`]s are
    /// currently based on draw order not depth. TODO replace this with a DepthLayer.
    #[doc(hidden)]
    pub fn enable_depth_test(&self) {
        self.gl.enable(Gl::DEPTH_TEST);
    }

    #[doc(hidden)]
    pub fn enable_cull_face(&self) {
        self.gl.enable(Gl::CULL_FACE)
    }

    /// Sets the background color to RGBA with components 0.0-1.0. Will take effect at the start of
    /// the next render.
    pub fn set_background_color(&mut self, color: Vec4) {
        self.gl.clear_color(color.x, color.y, color.z, color.w);
    }

    /// Not useful outside renderer. Use a framebuffer instead.
    pub(crate) fn set_viewport(&self, viewport: UVec2) {
        let size = viewport.as_ivec2();
        self.gl.viewport(0, 0, size.x, size.y);
    }

    /// Resets caches with latest information and calls [`Layer::pre_prepare`].
    #[doc(hidden)]
    pub fn pre_prepare(&mut self, layer: &mut impl Layer<C>, time_seconds: f32) {
        self.cached_canvas_size.set(None);
        self.time_delta = time_seconds - self.time;
        self.time = time_seconds;

        layer.pre_prepare(self);
    }

    /// Calls [`Layer::pre_render`], sets viewport, clears screen and calls [`Layer::render`].
    #[doc(hidden)]
    pub fn render(&mut self, layer: &mut impl Layer<C>) {
        // Pre-render such as allocating textures.
        layer.pre_render(self);

        // Set viewport and clear last frame to background color.
        self.set_viewport(self.canvas_size());
        self.gl.clear(Gl::COLOR_BUFFER_BIT);

        // Render everything.
        layer.render(self);
    }
}

/// Converts a viewport to an aspect ratio.
pub fn viewport_to_aspect(viewport: UVec2) -> f32 {
    let [width, height] = viewport.as_vec2().to_array();
    width / height
}
