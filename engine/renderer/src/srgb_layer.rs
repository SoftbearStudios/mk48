// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    DefaultRender, Framebuffer, Layer, RenderLayer, Renderer, Shader, TextureFormat, TriangleBuffer,
};
use glam::{vec2, Vec2};

/// Fast approximate antialiasing.
#[cfg(not(feature = "webgl2"))]
struct Fxaa {
    framebuffer: Framebuffer,
    shader: Shader,
}

#[cfg(not(feature = "webgl2"))]
impl DefaultRender for Fxaa {
    fn new(renderer: &Renderer) -> Self {
        // Requires its own framebuffer because fxaa operates on non-srgb colors.
        let framebuffer = Framebuffer::new2(renderer, false, TextureFormat::Rgba, false);
        let shader = renderer.create_shader(
            include_str!("shaders/fxaa.vert"),
            include_str!("shaders/fxaa.frag"),
        );
        Self {
            framebuffer,
            shader,
        }
    }
}

#[cfg(not(feature = "webgl2"))]
impl Fxaa {
    fn render(&mut self, renderer: &Renderer, binding: &crate::TriangleBufferBinding<Vec2, u16>) {
        if let Some(shader) = self.shader.bind(renderer) {
            shader.uniform("uVP", renderer.canvas_size().as_vec2());
            shader.uniform("uInverseVP", renderer.canvas_size().as_vec2().recip());
            shader.uniform("uSampler", self.framebuffer.as_texture());
            binding.draw();
        }
    }
}

/// Draws its inner [`Layer`] in the [SRGB color space](https://en.wikipedia.org/wiki/SRGB). It's
/// automatically added as the root layer if the `srgb` feature is enabled.
pub(crate) struct SrgbLayer<I> {
    buffer: TriangleBuffer<Vec2>,
    #[cfg(not(feature = "webgl2"))]
    fxaa: Option<Fxaa>,
    /// The inner [`Layer`] passed to [`new`][`Self::new`].
    pub inner: I,
    #[cfg(feature = "webgl2")]
    msaa: Option<Framebuffer>,
    shader: Shader,
    texture_fb: Framebuffer,
}

impl<I: Layer + DefaultRender> DefaultRender for SrgbLayer<I> {
    fn new(renderer: &Renderer) -> Self {
        Self::with_inner(renderer, DefaultRender::new(renderer))
    }
}

impl<I: Layer> SrgbLayer<I> {
    /// Creates a new [`SrgbLayer`].
    pub(crate) fn with_inner(renderer: &Renderer, inner: I) -> Self {
        let background_color = renderer.background_color;
        let antialias = renderer.antialias;
        let depth_stencil = I::DEPTH;

        // Use builtin msaa if possible (WebGL2 only).
        #[cfg(feature = "webgl2")]
        let msaa = antialias
            .then(|| Framebuffer::new_antialiased(renderer, background_color, depth_stencil));

        // Otherwise use postprocessing fxaa (WebGL doesn't support builtin aa).
        #[cfg(not(feature = "webgl2"))]
        let fxaa = antialias.then(|| Fxaa::new(renderer));

        // Create a buffer that has 1 triangle covering the whole screen.
        let mut buffer = TriangleBuffer::new(renderer);
        buffer.buffer(
            renderer,
            &[vec2(-1.0, 3.0), vec2(-1.0, -1.0), vec2(3.0, -1.0)],
            &[],
        );

        // For drawing to main screen.
        let shader = renderer.create_shader(
            include_str!("shaders/srgb.vert"),
            include_str!("shaders/srgb.frag"),
        );
        let texture_fb = Framebuffer::new2(
            renderer,
            [0; 4],
            false,
            TextureFormat::Srgba { premultiply: false },
            depth_stencil && !cfg!(feature = "webgl2"),
        );

        Self {
            buffer,
            #[cfg(not(feature = "webgl2"))]
            fxaa,
            inner,
            #[cfg(feature = "webgl2")]
            msaa,
            shader,
            texture_fb,
        }
    }
}

impl<I: Layer> Layer for SrgbLayer<I> {
    fn pre_prepare(&mut self, renderer: &Renderer) {
        self.inner.pre_prepare(renderer);
    }

    fn pre_render(&mut self, renderer: &Renderer) {
        self.inner.pre_render(renderer);
        let viewport = renderer.canvas_size();

        #[cfg(feature = "webgl2")]
        if let Some(msaa) = &mut self.msaa {
            msaa.set_viewport(renderer, viewport);
        }
        #[cfg(not(feature = "webgl2"))]
        if let Some(fxaa) = &mut self.fxaa {
            fxaa.framebuffer.set_viewport(renderer, viewport);
        }
        self.texture_fb.set_viewport(renderer, viewport);
    }
}

impl<I: RenderLayer<P>, P> RenderLayer<P> for SrgbLayer<I> {
    fn render(&mut self, renderer: &Renderer, params: P) {
        #[cfg(feature = "webgl2")]
        let binding = self.msaa.as_mut().map(|m| m.bind(renderer));
        #[cfg(not(feature = "webgl2"))]
        let binding = None;

        // Render directly to texture_fb if we aren't doing msaa.
        let fb = binding.unwrap_or_else(|| self.texture_fb.bind(renderer));

        // Need to clear since rendering to framebuffer (canvas has preserveDrawingBuffer: false).
        fb.clear();
        self.inner.render(renderer, params);

        drop(fb);

        // Downsample msaa results to texture_fb.
        #[cfg(feature = "webgl2")]
        if let Some(msaa) = &self.msaa {
            msaa.blit_to(renderer, Some(&mut self.texture_fb));
        }

        // Capture main screen draw and render to fxaa fb.
        #[cfg(not(feature = "webgl2"))]
        let fb = self.fxaa.as_mut().map(|f| f.framebuffer.bind(renderer));

        // Fxaa also requires this binding so bind before shader.
        let binding = self.buffer.bind(renderer);

        // Draw to main screen. Can't do `self.write_fb.blit_to(renderer, None);` because it
        // won't keep srgb encoding.
        if let Some(shader) = self.shader.bind(renderer) {
            shader.uniform("uSampler", self.texture_fb.as_texture());
            binding.draw();
        }

        // Draw fxaa framebuffer to main screen with fxaa applied.
        #[cfg(not(feature = "webgl2"))]
        {
            drop(fb);
            if let Some(fxaa) = &mut self.fxaa {
                fxaa.render(renderer, &binding);
            }
        }
    }
}
