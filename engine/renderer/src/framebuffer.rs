// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::gl::*;
use crate::renderer::Renderer;
use crate::rgba_array;
use crate::texture::{Texture, TextureFormat};
use glam::{UVec2, Vec4};
use std::rc::Rc;
use web_sys::{WebGlFramebuffer, WebGlRenderbuffer};

enum ColorBuffer {
    Texture(Texture),
    Renderbuffer(WebGlRenderbuffer),
}

enum DepthStencilBuffer {
    #[cfg(feature = "depth_texture")]
    Texture(Texture),
    Renderbuffer(WebGlRenderbuffer),
}

/// An offscreen [`Texture`] that you can draw to.
pub struct Framebuffer {
    background_color: [u8; 4],
    color: ColorBuffer,
    depth_stencil: Option<DepthStencilBuffer>,
    dimensions: UVec2,
    framebuffer: Rc<WebGlFramebuffer>, // For cheap clones for restoring previous.
    #[cfg(feature = "srgb")]
    srgb: bool,
}

impl Framebuffer {
    /// Creates a new [`Framebuffer`]. `liner_filter` specifies if it's
    /// [texture][`Self::as_texture`] uses linear filtering. Its [`TextureFormat`] is
    /// [`COLOR_RGBA`][`TextureFormat::COLOR_RGBA`].
    pub fn new(renderer: &Renderer, background_color: [u8; 4], linear_filter: bool) -> Self {
        Self::new2(
            renderer,
            background_color,
            linear_filter,
            TextureFormat::COLOR_RGBA,
            false,
        )
    }

    /// Creates a new [`Framebuffer`] that renders its depth to a texture.
    #[cfg(feature = "depth_texture")]
    pub fn new_depth(renderer: &Renderer) -> Self {
        Self::new_inner(renderer, [0; 4], false, None, true, true)
    }

    /// Like [`new`][`Self::new`] but with more options.
    pub(crate) fn new2(
        renderer: &Renderer,
        background_color: [u8; 4],
        linear_filter: bool,
        format: TextureFormat,
        depth_stencil: bool,
    ) -> Self {
        debug_assert!(
            !format.premultiply_alpha(),
            "pre-multiply not supported by framebuffer"
        );
        let texture = Texture::new_empty(renderer, format, linear_filter);
        Self::new_inner(
            renderer,
            background_color,
            format.is_srgb(),
            Some(texture),
            depth_stencil,
            false,
        )
    }

    /// Creates a new [`Framebuffer`] that's antialiased but isn't a texture.
    #[cfg(feature = "webgl2")]
    #[cfg_attr(not(feature = "srgb"), allow(unused))]
    pub(crate) fn new_antialiased(
        renderer: &Renderer,
        background_color: [u8; 4],
        depth_stencil: bool,
    ) -> Self {
        Self::new_inner(
            renderer,
            background_color,
            cfg!(feature = "srgb"),
            None,
            depth_stencil,
            false,
        )
    }

    fn new_inner(
        renderer: &Renderer,
        background_color: [u8; 4],
        srgb: bool,
        texture: Option<Texture>,
        depth_stencil: bool,
        depth_texture: bool,
    ) -> Self {
        let gl = &renderer.gl;

        // Create framebuffer but don't bind it yet.
        let framebuffer = gl.create_framebuffer().unwrap();

        let color = if let Some(texture) = texture {
            ColorBuffer::Texture(texture)
        } else {
            ColorBuffer::Renderbuffer(gl.create_renderbuffer().unwrap())
        };

        let depth_stencil = depth_stencil.then(|| {
            if depth_texture {
                #[cfg(not(feature = "depth_texture"))]
                unreachable!();
                #[cfg(feature = "depth_texture")]
                DepthStencilBuffer::Texture(Texture::new_empty(
                    renderer,
                    TextureFormat::Depth,
                    cfg!(feature = "webgl2"),
                ))
            } else {
                DepthStencilBuffer::Renderbuffer(gl.create_renderbuffer().unwrap())
            }
        });

        debug_assert!(cfg!(feature = "srgb") || !srgb, "sRGB unimplemented");

        let mut ret = Self {
            background_color,
            color,
            depth_stencil,
            dimensions: UVec2::ZERO,
            framebuffer: Rc::new(framebuffer),
            #[cfg(feature = "srgb")]
            srgb,
        };

        // Android WebGL can, contrary to the spec, silently error without the initial allocation.
        ret.set_viewport(renderer, UVec2::splat(1));

        let binding = FramebufferBinding::new(renderer, &ret);
        match &ret.color {
            ColorBuffer::Texture(texture) => {
                gl.framebuffer_texture_2d(
                    Gl::FRAMEBUFFER,
                    Gl::COLOR_ATTACHMENT0,
                    Gl::TEXTURE_2D,
                    Some(texture.inner()),
                    0,
                );
            }
            ColorBuffer::Renderbuffer(renderbuffer) => {
                gl.framebuffer_renderbuffer(
                    Gl::FRAMEBUFFER,
                    Gl::COLOR_ATTACHMENT0,
                    Gl::RENDERBUFFER,
                    Some(renderbuffer),
                );
            }
        }

        match &ret.depth_stencil {
            #[cfg(feature = "depth_texture")]
            Some(DepthStencilBuffer::Texture(texture)) => {
                gl.framebuffer_texture_2d(
                    Gl::FRAMEBUFFER,
                    Gl::DEPTH_ATTACHMENT, // TODO support stencil textures.
                    Gl::TEXTURE_2D,
                    Some(texture.inner()),
                    0,
                );
            }
            Some(DepthStencilBuffer::Renderbuffer(renderbuffer)) => {
                gl.framebuffer_renderbuffer(
                    Gl::FRAMEBUFFER,
                    Gl::DEPTH_STENCIL_ATTACHMENT,
                    Gl::RENDERBUFFER,
                    Some(&renderbuffer),
                );
            }
            None => {}
        }

        debug_assert_eq!(
            match gl.check_framebuffer_status(Gl::FRAMEBUFFER) {
                Gl::FRAMEBUFFER_INCOMPLETE_ATTACHMENT => Some("incomplete attachment"),
                Gl::FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT =>
                    Some("incomplete missing attachment"),
                Gl::FRAMEBUFFER_INCOMPLETE_DIMENSIONS => Some("incomplete dimensions"),
                Gl::FRAMEBUFFER_UNSUPPORTED => Some("unsupported"),
                #[cfg(feature = "webgl2")]
                Gl::FRAMEBUFFER_INCOMPLETE_MULTISAMPLE => Some("incomplete multisample"),
                #[cfg(feature = "webgl2")]
                Gl::RENDERBUFFER_SAMPLES => Some("samples"),
                _ => None,
            },
            None
        );

        drop(binding);
        ret
    }

    /// Sets the dimensions of the [`Framebuffer`]. If you want to render a whole screen,
    /// `viewport` should be [`Renderer::canvas_size`].
    ///
    /// Preferably call this before [`RenderLayer::render`][`crate::renderer::RenderLayer::render`]
    /// to avoid stalling the rendering pipeline if viewport changes between frames.
    ///
    /// NOTE: this clears the [`Framebuffer`] if the viewport changes between calls.
    pub fn set_viewport(&mut self, renderer: &Renderer, viewport: UVec2) {
        if viewport != self.dimensions {
            // TODO is this required?
            let restore = RestoreFramebuffer::new(renderer);

            self.dimensions = viewport;

            let resize_renderbuffer = |r: &WebGlRenderbuffer, format: u32, ms: bool| {
                let gl = &renderer.gl;

                // bind renderbuffer ->>
                gl.bind_renderbuffer(Gl::RENDERBUFFER, Some(r));

                let d = viewport.as_ivec2();

                if ms && renderer.antialias {
                    #[cfg(not(feature = "webgl2"))]
                    unimplemented!();
                    #[cfg(feature = "webgl2")]
                    {
                        let max_samples = renderer.max_samples();
                        gl.renderbuffer_storage_multisample(
                            Gl::RENDERBUFFER,
                            max_samples,
                            format,
                            d.x,
                            d.y,
                        );
                    }
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
                #[allow(unused_variables)]
                ColorBuffer::Renderbuffer(r) => {
                    #[warn(unused_variables)]
                    match self.depth_stencil {
                        #[cfg(feature = "depth_texture")]
                        Some(DepthStencilBuffer::Texture(_)) => {
                            // This format doesn't matter, it just has to be supported by all
                            // browsers to make the Framebuffer complete.
                            let format = Gl::RGBA4;
                            resize_renderbuffer(r, format, false)
                        }

                        _ => {
                            #[cfg(feature = "srgb")]
                            let format = Srgb::SRGB8_ALPHA8_EXT;

                            // WebGL can only have 16 bit color without EXT_sRGB.
                            #[cfg(not(feature = "srgb"))]
                            let format = Gl::RGBA4;

                            resize_renderbuffer(r, format, cfg!(feature = "webgl2"))
                        }
                    }
                }
            }

            match &mut self.depth_stencil {
                #[cfg(feature = "depth_texture")]
                Some(DepthStencilBuffer::Texture(texture)) => {
                    texture.realloc_with_opt_bytes(renderer, viewport, None);
                }
                Some(DepthStencilBuffer::Renderbuffer(r)) => {
                    let format;
                    #[cfg(not(feature = "webgl2"))]
                    {
                        format = Gl::DEPTH_STENCIL;
                    }
                    #[cfg(feature = "webgl2")]
                    {
                        format = Gl::DEPTH24_STENCIL8;
                    }

                    resize_renderbuffer(r, format, cfg!(feature = "webgl2"));
                }
                None => {}
            }

            drop(restore);
        }
    }

    /// Binds the [`Framebuffer`], causing all
    /// [`draw`][`crate::buffer::TriangleBufferBinding::draw`]s to draw to it.
    ///
    /// NOTE: Does not get cleared between frames.
    #[must_use]
    pub fn bind<'a>(&'a mut self, renderer: &'a Renderer) -> FramebufferBinding<'a> {
        FramebufferBinding::new(renderer, self)
    }

    /// Gets the texture that the [`Framebuffer`] renders to.
    pub fn as_texture(&self) -> &Texture {
        match &self.color {
            ColorBuffer::Texture(texture) => texture,
            ColorBuffer::Renderbuffer(_) => panic!("not texture"),
        }
    }

    /// Gets the depth texture that the [`Framebuffer`] renders to.
    #[cfg(feature = "depth_texture")]
    pub fn as_depth_texture(&self) -> &Texture {
        match &self.depth_stencil {
            Some(DepthStencilBuffer::Texture(texture)) => texture,
            Some(DepthStencilBuffer::Renderbuffer(_)) => panic!("not texture"),
            None => panic!("no depth/stencil attachment"),
        }
    }

    /// Copies the color buffer to another [`Framebuffer`].
    #[cfg(feature = "webgl2")]
    pub fn blit_to(&self, renderer: &Renderer, other: Option<&mut Self>) {
        let gl = &renderer.gl;

        // Bind read and write. RestoreFramebuffer not needed since not changing Gl::FRAMEBUFFER.
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

/// A bound [`Framebuffer`] that will be capture all draws. You can use the results with
/// [`Framebuffer::as_texture`].
pub struct FramebufferBinding<'a> {
    renderer: &'a Renderer,
    _framebuffer: &'a Framebuffer,
}

impl<'a> FramebufferBinding<'a> {
    fn new(renderer: &'a Renderer, framebuffer: &'a Framebuffer) -> Self {
        let owned = OwnedFramebufferBinding {
            framebuffer: Rc::clone(&framebuffer.framebuffer),
            dimensions: framebuffer.dimensions,
        };
        OwnedFramebufferBinding::restore(renderer, Some(&owned));
        let mut bindings = renderer.bound_framebuffers.take();
        bindings.push(owned);
        renderer.bound_framebuffers.set(bindings);

        Self {
            renderer,
            _framebuffer: framebuffer,
        }
    }

    /// Clears the [`Framebuffer`].
    pub fn clear(&self) {
        let color = Vec4::from(
            self._framebuffer
                .background_color
                .map(|c| c as f32 * (1.0 / 255.0)),
        );
        #[cfg(feature = "srgb")]
        let color = if self._framebuffer.srgb {
            rgba_array(self._framebuffer.background_color)
        } else {
            color
        };
        self.renderer.clear(color);
    }
}

impl<'a> Drop for FramebufferBinding<'a> {
    fn drop(&mut self) {
        let renderer = self.renderer;

        let mut bindings = renderer.bound_framebuffers.take();
        let _ = bindings.pop();

        OwnedFramebufferBinding::restore(renderer, bindings.last());

        renderer.bound_framebuffers.set(bindings);
    }
}

/// For restoring [`FramebufferBinding`]s if multiple are bound recursively. Uses an [`Rc`] for
/// cheap clones.
pub(crate) struct OwnedFramebufferBinding {
    framebuffer: Rc<WebGlFramebuffer>,
    dimensions: UVec2,
}

impl OwnedFramebufferBinding {
    fn restore(renderer: &Renderer, me: Option<&Self>) {
        if let Some(me) = me {
            // Set viewport and bind framebuffer.
            renderer.set_viewport(me.dimensions);
            renderer
                .gl
                .bind_framebuffer(Gl::FRAMEBUFFER, Some(&me.framebuffer));
        } else {
            // Reset viewport and unbind framebuffer.
            renderer.set_viewport(renderer.canvas_size()); // TODO dedup calls with same value.
            renderer.gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
        }
    }
}

/// Required when changing Gl::FRAMEBUFFER in a function.
struct RestoreFramebuffer<'a> {
    renderer: &'a Renderer,
}

impl<'a> RestoreFramebuffer<'a> {
    fn new(renderer: &'a Renderer) -> Self {
        Self { renderer }
    }
}

impl<'a> Drop for RestoreFramebuffer<'a> {
    fn drop(&mut self) {
        let renderer = self.renderer;
        let bindings = renderer.bound_framebuffers.take();
        OwnedFramebufferBinding::restore(renderer, bindings.last());
        renderer.bound_framebuffers.set(bindings);
    }
}
