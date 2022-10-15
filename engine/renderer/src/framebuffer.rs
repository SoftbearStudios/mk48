// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::gl::*;
use crate::renderer::Renderer;
use crate::texture::{Texture, TextureFormat};
use glam::UVec2;
use web_sys::WebGlFramebuffer;

#[cfg(feature = "srgb")]
use web_sys::WebGlRenderbuffer;

enum ColorBuffer {
    Texture(Texture),
    #[cfg(feature = "srgb")]
    Renderbuffer(WebGlRenderbuffer),
}

/// An offscreen [`Texture`] that you can draw to.
pub struct Framebuffer {
    color: ColorBuffer,
    #[cfg(feature = "srgb")]
    depth_stencil: Option<WebGlRenderbuffer>,
    dimensions: UVec2,
    framebuffer: WebGlFramebuffer,
}

impl Framebuffer {
    /// Creates a new [`Framebuffer`]. `liner_filter` specifies if it's
    /// [texture][`Self::as_texture`] uses linear filtering.
    pub fn new<C>(renderer: &Renderer<C>, linear_filter: bool) -> Self {
        Self::new2(renderer, linear_filter, TextureFormat::Rgba, false)
    }

    /// Like [`new`][`Self::new`] but with more options.
    pub(crate) fn new2<C>(
        renderer: &Renderer<C>,
        linear_filter: bool,
        format: TextureFormat,
        depth_stencil: bool,
    ) -> Self {
        let texture = Texture::new_empty(renderer, format, linear_filter);
        Self::new_inner(renderer, Some(texture), depth_stencil)
    }

    /// Creates a new [`Framebuffer`] that is antialiased but isn't a texture.
    #[cfg(all(feature = "srgb", feature = "webgl2"))]
    pub(crate) fn new_antialiased<C>(renderer: &Renderer<C>, depth_stencil: bool) -> Self {
        Self::new_inner(renderer, None, depth_stencil)
    }

    fn new_inner<C>(renderer: &Renderer<C>, texture: Option<Texture>, depth_stencil: bool) -> Self {
        let gl = &renderer.gl;

        // Create framebuffer and bind texture to it.
        let framebuffer = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&framebuffer));

        let color = if let Some(texture) = texture {
            gl.framebuffer_texture_2d(
                Gl::FRAMEBUFFER,
                Gl::COLOR_ATTACHMENT0,
                Gl::TEXTURE_2D,
                Some(texture.inner()),
                0,
            );
            ColorBuffer::Texture(texture)
        } else {
            #[cfg(not(feature = "srgb"))]
            unreachable!();
            #[cfg(feature = "srgb")]
            {
                let renderbuffer = gl.create_renderbuffer().unwrap();

                // Bind unbind and renderbuffer.
                renderer
                    .gl
                    .bind_renderbuffer(Gl::RENDERBUFFER, Some(&renderbuffer));
                renderer.gl.bind_renderbuffer(Gl::RENDERBUFFER, None);

                gl.framebuffer_renderbuffer(
                    Gl::FRAMEBUFFER,
                    Gl::COLOR_ATTACHMENT0,
                    Gl::RENDERBUFFER,
                    Some(&renderbuffer),
                );

                ColorBuffer::Renderbuffer(renderbuffer)
            }
        };

        #[cfg(not(feature = "srgb"))]
        assert!(!depth_stencil);
        #[cfg(feature = "srgb")]
        let depth_stencil = depth_stencil.then(|| {
            let renderbuffer = gl.create_renderbuffer().unwrap();

            // Bind unbind and renderbuffer.
            renderer
                .gl
                .bind_renderbuffer(Gl::RENDERBUFFER, Some(&renderbuffer));
            renderer.gl.bind_renderbuffer(Gl::RENDERBUFFER, None);

            gl.framebuffer_renderbuffer(
                Gl::FRAMEBUFFER,
                Gl::DEPTH_STENCIL_ATTACHMENT,
                Gl::RENDERBUFFER,
                Some(&renderbuffer),
            );

            renderbuffer
        });

        // Must unbind framebuffer (unlike many other unbinds) or draws would go to it.
        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);

        Self {
            color,
            #[cfg(feature = "srgb")]
            depth_stencil,
            dimensions: UVec2::ZERO,
            framebuffer,
        }
    }

    /// Sets the dimensions of the [`Framebuffer`]. If you want to render a whole screen,
    /// `viewport` should be [`Renderer::canvas_size`].
    ///
    /// Preferably call this before [`Layer::render`][`crate::renderer::Layer::render`] to not stall
    /// the rendering pipeline if viewport changes between frames.
    ///
    /// NOTE: this clears the [`Framebuffer`].
    pub fn set_viewport<C>(&mut self, renderer: &Renderer<C>, viewport: UVec2) {
        if viewport != self.dimensions {
            self.dimensions = viewport;
            #[cfg(feature = "srgb")]
            let resize_renderbuffer = |r: &WebGlRenderbuffer, format: u32| {
                let gl = &renderer.gl;

                // bind renderbuffer ->>
                gl.bind_renderbuffer(Gl::RENDERBUFFER, Some(r));

                let d = viewport.as_ivec2();
                #[cfg(not(feature = "webgl2"))]
                gl.renderbuffer_storage(Gl::RENDERBUFFER, format, d.x, d.y);
                #[cfg(feature = "webgl2")]
                if let Some(max_samples) = renderer.antialiasing {
                    gl.renderbuffer_storage_multisample(
                        Gl::RENDERBUFFER,
                        max_samples,
                        format,
                        d.x,
                        d.y,
                    );
                } else {
                    gl.renderbuffer_storage(Gl::RENDERBUFFER, format, d.x, d.y);
                }

                // <-- unbind renderbuffer
                gl.bind_renderbuffer(Gl::RENDERBUFFER, None);
            };

            match &mut self.color {
                ColorBuffer::Texture(texture) => {
                    texture.realloc_with_opt_bytes(renderer, viewport, None);
                }
                #[cfg(feature = "srgb")]
                ColorBuffer::Renderbuffer(r) => resize_renderbuffer(r, Srgb::SRGB8_ALPHA8_EXT),
            }

            #[cfg(feature = "srgb")]
            if let Some(depth_stencil) = &mut self.depth_stencil {
                let format;
                #[cfg(not(feature = "webgl2"))]
                {
                    format = Gl::DEPTH_STENCIL;
                }
                #[cfg(feature = "webgl2")]
                {
                    format = Gl::DEPTH24_STENCIL8;
                }

                resize_renderbuffer(depth_stencil, format);
            }
        }
    }

    /// Binds the [`Framebuffer`], causing all
    /// [`draw`][`crate::buffer::TriangleBufferBinding::draw`]s to draw to it.
    ///
    /// NOTE: Does not get cleared between frames.
    pub fn bind<'a, C>(&'a mut self, renderer: &'a Renderer<C>) -> FramebufferBinding<'a, C> {
        FramebufferBinding::new(renderer, self)
    }

    /// Gets the texture that the [`Framebuffer`] renders to.
    pub fn as_texture(&self) -> &Texture {
        match &self.color {
            ColorBuffer::Texture(texture) => texture,
            #[cfg(feature = "srgb")]
            ColorBuffer::Renderbuffer(_) => panic!("not texture"),
        }
    }

    /// Doesn't actually require srgb but is unused if not srgb.
    #[cfg(all(feature = "webgl2", feature = "srgb"))]
    pub(crate) fn blit_to<C>(&self, renderer: &Renderer<C>, other: Option<&mut Self>) {
        let gl = &renderer.gl;

        // Bind read and write.
        gl.bind_framebuffer(Gl::READ_FRAMEBUFFER, Some(&self.framebuffer));
        if let Some(other) = &other {
            gl.bind_framebuffer(Gl::DRAW_FRAMEBUFFER, Some(&other.framebuffer));
        }

        // Blit framebuffers.
        let from = self.dimensions.as_ivec2();
        let to = if let Some(other) = &other {
            other.dimensions
        } else {
            renderer.canvas_size()
        }
        .as_ivec2();
        assert_eq!(from, to);

        gl.blit_framebuffer(
            0,
            0,
            from.x,
            from.y,
            0,
            0,
            to.x,
            to.y,
            Gl::COLOR_BUFFER_BIT,
            Gl::NEAREST,
        );

        // Unbind read and write.
        if other.is_some() {
            gl.bind_framebuffer(Gl::DRAW_FRAMEBUFFER, None);
        }
        gl.bind_framebuffer(Gl::READ_FRAMEBUFFER, None);
    }
}

/// A bound [`Framebuffer`] that will be capture all draws. You can use results with
/// [`Framebuffer::as_texture`].
pub struct FramebufferBinding<'a, C> {
    renderer: &'a Renderer<C>,
    _framebuffer: &'a Framebuffer,
}

impl<'a, C> FramebufferBinding<'a, C> {
    fn new(renderer: &'a Renderer<C>, framebuffer: &'a Framebuffer) -> Self {
        // Set viewport and bind framebuffer.
        renderer.set_viewport(framebuffer.dimensions);
        renderer
            .gl
            .bind_framebuffer(Gl::FRAMEBUFFER, Some(&framebuffer.framebuffer));

        Self {
            renderer,
            _framebuffer: framebuffer,
        }
    }

    // TODO fn clear(&self).
}

impl<'a, C> Drop for FramebufferBinding<'a, C> {
    fn drop(&mut self) {
        // Reset viewport and unbind framebuffer.
        self.renderer.set_viewport(self.renderer.canvas_size());
        self.renderer.gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
    }
}
