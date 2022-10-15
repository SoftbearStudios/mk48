// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::gl::*;
use crate::{Framebuffer, Layer, Renderer, Shader, TextureFormat, TriangleBuffer};
use glam::{vec2, Vec2};

/// Fast approximate antialiasing.
#[cfg(not(feature = "webgl2"))]
struct Fxaa {
    framebuffer: Framebuffer,
    shader: Shader,
}

#[cfg(not(feature = "webgl2"))]
impl Fxaa {
    fn new<C>(renderer: &Renderer<C>) -> Self {
        let framebuffer = Framebuffer::new(renderer, false);
        let shader = renderer.create_shader(
            include_str!("shaders/fxaa.vert"),
            include_str!("shaders/fxaa.frag"),
        );
        Self {
            framebuffer,
            shader,
        }
    }

    fn render<C>(
        &mut self,
        renderer: &Renderer<C>,
        binding: &crate::TriangleBufferBinding<Vec2, u16>,
    ) {
        if let Some(shader) = self.shader.bind(renderer) {
            shader.uniform2f("uVP", renderer.canvas_size().as_vec2());
            shader.uniform2f("uInverseVP", renderer.canvas_size().as_vec2().recip());
            shader.uniform_texture("uSampler", self.framebuffer.as_texture(), 0);
            binding.draw();
        }
    }
}

/// Draws its inner [`Layer`] in the [SRGB color space](https://en.wikipedia.org/wiki/SRGB).
pub struct SrgbLayer<I> {
    #[cfg(feature = "webgl2")]
    antialiased_fb: Framebuffer,
    buffer: TriangleBuffer<Vec2>,
    #[cfg(not(feature = "webgl2"))]
    fxaa: Option<Fxaa>,
    /// The inner [`Layer`] passed to [`new`][`Self::new`].
    pub inner: I,
    shader: Shader,
    texture_fb: Framebuffer,
}

impl<I> SrgbLayer<I> {
    /// Creates a new [`SrgbLayer`].
    pub fn new<C>(renderer: &Renderer<C>, inner: I) -> Self {
        let texture_fb_depth_stencil = cfg!(not(feature = "webgl2"));
        // For drawing with anti-aliasing.
        #[cfg(feature = "webgl2")]
        let antialiased_fb = Framebuffer::new_antialiased(renderer, true);

        #[cfg(not(feature = "webgl2"))]
        let fxaa = renderer.antialiasing.map(|_| Fxaa::new(renderer));

        // Create a buffer that 1 triangle.
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
            false,
            TextureFormat::Srgba,
            texture_fb_depth_stencil,
        );

        Self {
            #[cfg(feature = "webgl2")]
            antialiased_fb,
            buffer,
            #[cfg(not(feature = "webgl2"))]
            fxaa,
            inner,
            shader,
            texture_fb,
        }
    }
}

impl<C, I: Layer<C>> Layer<C> for SrgbLayer<I> {
    fn pre_prepare(&mut self, renderer: &Renderer<C>) {
        self.inner.pre_prepare(renderer);
    }

    fn pre_render(&mut self, renderer: &Renderer<C>) {
        self.inner.pre_render(renderer);
        #[cfg(feature = "webgl2")]
        self.antialiased_fb
            .set_viewport(renderer, renderer.canvas_size());
        #[cfg(not(feature = "webgl2"))]
        if let Some(fxaa) = &mut self.fxaa {
            fxaa.framebuffer
                .set_viewport(renderer, renderer.canvas_size());
        }
        self.texture_fb
            .set_viewport(renderer, renderer.canvas_size());
    }

    fn render(&mut self, renderer: &Renderer<C>) {
        #[cfg(not(feature = "webgl2"))]
        let binding = self.texture_fb.bind(renderer);
        #[cfg(feature = "webgl2")]
        let binding = self.antialiased_fb.bind(renderer);

        renderer
            .gl
            .clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT | Gl::STENCIL_BUFFER_BIT);
        self.inner.render(renderer);
        drop(binding);

        // Draw to main screen. Can't do `self.write_fb.blit_to(renderer, None);` because it
        // won't keep srgb encoding.
        #[cfg(feature = "webgl2")]
        self.antialiased_fb
            .blit_to(renderer, Some(&mut self.texture_fb));

        let binding = self.buffer.bind(renderer);

        #[cfg(not(feature = "webgl2"))]
        let fb = self.fxaa.as_mut().map(|f| f.framebuffer.bind(renderer));

        if let Some(shader) = self.shader.bind(renderer) {
            shader.uniform_texture("uSampler", self.texture_fb.as_texture(), 0);
            binding.draw();
        }

        #[cfg(not(feature = "webgl2"))]
        {
            drop(fb);
            if let Some(fxaa) = &mut self.fxaa {
                fxaa.render(renderer, &binding);
            }
        }
    }
}
