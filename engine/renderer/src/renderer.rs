// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::gl::*;
use crate::shader::Shader;
use crate::OwnedFramebufferBinding;
pub use engine_macros::Layer;
use glam::{uvec2, UVec2, Vec4};
use js_hooks::error_message;
use linear_map::LinearMap;
use std::cell::{Cell, RefCell};
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

#[cfg(feature = "srgb")]
use crate::srgb_layer::SrgbLayer;

/// Owns a [`Renderer`] and a [`Layer`].
pub struct RenderChain<L> {
    #[cfg(not(feature = "srgb"))]
    layer: L,
    #[cfg(feature = "srgb")]
    layer: SrgbLayer<L>,
    renderer: Renderer,
}

impl<L: Layer> RenderChain<L> {
    /// Creates a new [`RenderChain`] from a `renderer` and a `layer`.
    pub fn new(
        background_color: [u8; 4],
        antialias: bool,
        f: impl FnOnce(&mut Renderer) -> L,
    ) -> Result<Self, String> {
        // Don't give backbuffer aa and depth buffer if we aren't using it.
        let backbuffer = cfg!(not(feature = "srgb"));
        let mut renderer = Renderer::new(
            background_color,
            antialias,
            backbuffer,
            backbuffer && L::ALPHA,
            backbuffer && L::DEPTH,
            backbuffer && L::STENCIL,
        )?;

        let layer = f(&mut renderer);

        #[cfg(not(feature = "srgb"))]
        return Ok(Self { layer, renderer });
        #[cfg(feature = "srgb")]
        Ok(Self {
            layer: SrgbLayer::with_inner(&renderer, layer),
            renderer,
        })
    }

    /// Gets the [`Layer`] passed to [`new`][`Self::new`] mutably.
    #[doc(hidden)]
    pub fn layer_mut(&mut self) -> &mut L {
        #[cfg(not(feature = "srgb"))]
        return &mut self.layer;
        #[cfg(feature = "srgb")]
        &mut self.layer.inner
    }

    /// Gets the [`Renderer`] passed to [`new`][`Self::new`].
    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    /// Gets the [`Renderer`] passed to [`new`][`Self::new`] mutably.
    #[doc(hidden)]
    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    /// Begins rendering a frame. Must call [`end`][`RenderFrame::end`] on result.
    #[must_use]
    pub fn begin(&mut self, time_seconds: f32) -> RenderFrame<'_, L> {
        self.renderer.pre_prepare(&mut self.layer, time_seconds);
        RenderFrame {
            layer: &mut self.layer,
            renderer: &self.renderer,
        }
    }
}

/// A single frame being rendered.
pub struct RenderFrame<'a, L> {
    #[cfg(not(feature = "srgb"))]
    layer: &'a mut L,
    #[cfg(feature = "srgb")]
    layer: &'a mut SrgbLayer<L>,
    renderer: &'a Renderer,
}

impl<'a, L> RenderFrame<'a, L> {
    /// Borrows the [`Renderer`] that is rendering the frame. and the [`Layer`] that is rendering to
    /// the frame.
    pub fn draw(&mut self) -> (&Renderer, &mut L) {
        #[cfg(not(feature = "srgb"))]
        return (self.renderer, &mut *self.layer);
        #[cfg(feature = "srgb")]
        (self.renderer, &mut self.layer.inner)
    }

    /// Ends the rendering of a frame by calling [`render`][`RenderLayer::render`] with `params`.
    pub fn end<P>(self, params: P)
    where
        L: RenderLayer<P>,
    {
        self.renderer.render(self.layer, params);
        std::mem::forget(self); // So drop doesn't panic.
    }
}

impl<L> Drop for RenderFrame<'_, L> {
    fn drop(&mut self) {
        panic!("must call RenderFrame::end")
    }
}

/// Like [`Default`] but requires a [`Renderer`].
pub trait DefaultRender {
    /// Like [`default`][`Default::default`] but requires a [`Renderer`].
    fn new(renderer: &Renderer) -> Self;
}

// TODO for #[derive(DefaultRender)].
impl<T: Default> DefaultRender for T {
    #[doc(hidden)]
    fn new(_: &Renderer) -> Self {
        Self::default()
    }
}

/// Contains things that can be drawn.
///
/// # Derive Layer
/// [`Layer`] be derived on structs that have fields which also implement [`Layer`]. If your struct
/// has non [`Layer`] fields you can label the [`Layer`] fields with `#[layer]`.
/// ```rust
/// #[derive(Layer)]
/// #[alpha] // If we need an alpha channel   (and OtherLayer doesn't require it).
/// #[depth] // If we need a depth buffer     ..
/// #[stencil] // If we need a stencil buffer ..
/// struct MyLayer {
///     #[layer]
///     other_layer: OtherLayer,
///     not_a_layer: Shader,
/// }
/// ```
///
/// # Derive RenderLayer
/// [`RenderLayer`] can be derived within the `#[derive(Layer)]` for now.
/// ```rust
/// #[derive(Layer)]
/// #[render(T)]
/// struct MyLayer {
///     #[layer]
///     other_layer: OtherLayer,
///     #[layer]
///     another_layer: AnotherLayer,
///     not_a_layer: Shader,
/// }
/// ```
/// `T` is the [`render`][`RenderLayer::render`]'s `params`.
///
/// [`borrow`][`std::borrow::Borrow::borrow`] will be called on `params` for each [`RenderLayer`] to
/// allow them to use a subset of `params` ([`borrow`][`std::borrow::Borrow::borrow`] allows
/// passthrough as well e.g. `fn borrow(&T) -> &T`).
///
/// TODO allow multiple `#[render(T)]`s with different types and allow labeling which [`Layer`]s use
/// each params.
pub trait Layer {
    /// If this [`Layer`] requires an alpha channel.
    const ALPHA: bool = false;

    /// If this [`Layer`] requires a depth buffer.
    const DEPTH: bool = false;

    /// If this [`Layer`] requires a stencil buffer.
    const STENCIL: bool = false;

    /// Called at the start of each frame. Useful for copying the [`Renderer`]'s time.
    fn pre_prepare(&mut self, renderer: &Renderer) {
        let _ = renderer;
    }

    /// Called right before any [`Layer`] is [`render`][`RenderLayer::render`]ed. Useful for
    /// creating textures.
    fn pre_render(&mut self, renderer: &Renderer) {
        let _ = renderer;
    }
}

/// Allows a [`Layer`] to be renderered with `params`. Layers can be rendered with different sets
/// of params (such as for rendering shadows etc.).
pub trait RenderLayer<P>: Layer {
    /// Renders the [`Layer`].
    fn render(&mut self, renderer: &Renderer, params: P);
}

/// An abstraction over
/// [WebGL](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.WebGlRenderingContext.html)/
/// [WebGL2](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.WebGl2RenderingContext.html)
/// that can be used in 2D and 3D applications.
pub struct Renderer {
    /// HTML Canvas.
    canvas: HtmlCanvasElement,
    cached_canvas_size: Cell<Option<UVec2>>,
    /// WebGL context.
    pub(crate) gl: Gl,
    /// WebGL extensions.
    pub(crate) aia: Option<Aia>,
    pub(crate) khr: Option<Khr>,
    pub(crate) ovao: Ovao,
    /// Save this in case we need to transform it later, for sRGB purposes.
    pub background_color: [u8; 4],
    /// Current time in seconds since start.
    pub time: f32,
    /// Seconds since last frame.
    pub time_delta: f32,
    /// Cache of static shaders.
    shader_cache: RefCell<LinearMap<(&'static str, &'static str), Shader>>,
    /// WebGL doesn't support antialiasing with srgb.
    #[allow(unused)]
    pub(crate) antialias: bool,
    /// How much anisotropy to use or None if shouldn't use.
    #[cfg(feature = "anisotropy")]
    pub(crate) anisotropy: Option<u32>,
    #[cfg(feature = "webgl2")]
    max_samples: Cell<Option<i32>>,
    /// To save redundant calls to `Gl::active_texture`.
    active_texture: Cell<u8>,
    /// To allow binding [`Framebuffer`][`crate::Framebuffer`]s recursively.
    pub(crate) bound_framebuffers: Cell<Vec<OwnedFramebufferBinding>>,
    pub(crate) current_clear_color: Cell<Vec4>,
}

impl Renderer {
    /// Creates a new WebGL/WebGL2 render, attaching it to the canvas element with the id "canvas."
    #[doc(hidden)]
    pub(crate) fn new(
        background_color: [u8; 4],
        antialias: bool,
        builtin_aa: bool,
        alpha: bool,
        depth: bool,
        stencil: bool,
    ) -> Result<Self, String> {
        let builtin_aa = builtin_aa && antialias;
        let _ = alpha; // TODO

        let options = js_sys::JSON::parse(&format!(
            r##"{{
            "alpha": false,
            "depth": {depth},
            "stencil": {stencil},
            "antialias": {builtin_aa},
            "power_preference": "high-performance",
            "preserve_drawing_buffer": false
        }}"##
        ))
        .unwrap();

        let canvas = js_hooks::canvas();
        // See: https://developer.mozilla.org/en-US/docs/Web/API/HTMLCanvasElement/getContext
        let gl = canvas
            .get_context_with_context_options(GL_NAME, &options)
            .map_err(|e| {
                error_message(&e)
                    .unwrap_or_else(|| concat!("Error initializing ", gl_title!()).into())
            })?
            .ok_or(concat!(gl_title!(), " unsupported"))?
            .dyn_into::<Gl>()
            .unwrap();

        let khr = gl
            .get_extension("KHR_parallel_shader_compile")
            .unwrap()
            .map(|_| KhrParallelShaderCompile);

        let ovao = gl.get_extension_ovao();

        // WebGL2 has these built in by default. In WebGL we only need to enable it, not save it.
        #[cfg(all(not(feature = "webgl2"), feature = "srgb"))]
        gl.get_extension("EXT_sRGB").unwrap().expect("no EXT_sRGB");
        #[cfg(all(not(feature = "webgl2"), feature = "depth_texture"))]
        gl.get_extension("WEBGL_depth_texture")
            .expect("no WEBGL_depth_texture");

        gl.enable(Gl::BLEND);

        // First argument is Gl::SRC_ALPHA if not premultiplied alpha, Gl::ONE if premultiplied(?).
        gl.blend_func(Gl::ONE, Gl::ONE_MINUS_SRC_ALPHA);

        Ok(Self {
            canvas,
            cached_canvas_size: Cell::new(None),
            gl,
            aia: None,
            khr,
            ovao,
            background_color,
            time: 0.0,
            time_delta: 0.0,
            shader_cache: Default::default(),
            antialias,
            #[cfg(feature = "anisotropy")]
            anisotropy: None,
            #[cfg(feature = "webgl2")]
            max_samples: Default::default(),
            active_texture: Default::default(),
            bound_framebuffers: Default::default(),
            current_clear_color: Default::default(),
        })
    }
}

impl Renderer {
    #[cfg(feature = "webgl2")]
    pub(crate) fn max_samples(&self) -> i32 {
        self.max_samples
            .update(|prev| {
                Some(prev.unwrap_or_else(|| {
                    (self
                        .gl
                        .get_parameter(Gl::MAX_SAMPLES)
                        .unwrap()
                        .as_f64()
                        .unwrap() as i32)
                        .min(8)
                }))
            })
            .unwrap()
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
    pub fn set_depth_test(&self, enabled: bool) {
        if enabled {
            self.gl.enable(Gl::DEPTH_TEST);
        } else {
            self.gl.disable(Gl::DEPTH_TEST);
        }
    }

    // TODO doc
    #[doc(hidden)]
    pub fn set_color_mask(&self, mask: bool) {
        self.gl.color_mask(mask, mask, mask, mask)
    }

    // TODO doc
    #[doc(hidden)]
    pub fn set_cull_face(&self, front: bool) {
        self.gl.cull_face(if front { Gl::FRONT } else { Gl::BACK });
    }

    // TODO doc
    #[doc(hidden)]
    pub fn enable_cull_face(&self) {
        self.gl.enable(Gl::CULL_FACE)
    }

    pub(crate) fn clear(&self, color: Vec4) {
        if color != self.current_clear_color.get() {
            self.current_clear_color.set(color);
            self.gl.clear_color(color.x, color.y, color.z, color.w);
        }
        self.gl
            .clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT | Gl::STENCIL_BUFFER_BIT);
    }

    // TODO replace with BlendLayer<T>.
    #[doc(hidden)]
    pub fn set_blend(&self, enabled: bool) {
        if enabled {
            self.gl.enable(Gl::BLEND);
        } else {
            self.gl.disable(Gl::BLEND);
        }
    }

    /// Sets the active texture to `index`. Doesn't repeat the `Gl::active_texture` call if it
    /// hasn't changed.
    pub(crate) fn active_texture(&self, index: usize) {
        assert!(index < 32, "only 32 textures supported");
        let index = index as u8;

        // Don't do redundant calls.
        if index == self.active_texture.get() {
            return;
        }
        self.active_texture.set(index);
        self.gl.active_texture(Gl::TEXTURE0 + index as u32);
    }

    /// Not useful outside renderer. Use a framebuffer instead.
    pub(crate) fn set_viewport(&self, viewport: UVec2) {
        let size = viewport.as_ivec2();
        self.gl.viewport(0, 0, size.x, size.y);
    }

    /// Resets caches with latest information and calls [`Layer::pre_prepare`].
    ///
    /// Use [`RenderChain`] instead of this method directly.
    pub(crate) fn pre_prepare(&mut self, layer: &mut impl Layer, time_seconds: f32) {
        self.cached_canvas_size.set(None);
        self.time_delta = time_seconds - self.time;
        self.time = time_seconds;

        layer.pre_prepare(self);
    }

    /// Calls [`Layer::pre_render`], sets viewport, clears screen and calls [`Layer::render`].
    ///
    /// Use [`RenderChain`] instead of this method directly.
    pub(crate) fn render<P>(&self, layer: &mut impl RenderLayer<P>, params: P) {
        // Pre-render such as allocating textures.
        layer.pre_render(self);

        // Set viewport and clear last frame to background color.
        self.set_viewport(self.canvas_size());
        self.clear(Vec4::from(
            self.background_color.map(|c| c as f32 * (1.0 / 255.0)),
        ));

        // Render everything.
        layer.render(self, params);
    }
}

/// Converts a viewport to an aspect ratio.
pub fn viewport_to_aspect(viewport: UVec2) -> f32 {
    let [width, height] = viewport.as_vec2().to_array();
    width / height
}
