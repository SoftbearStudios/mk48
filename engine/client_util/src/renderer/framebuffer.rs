use crate::renderer::renderer::Renderer;
use crate::renderer::texture::{Texture, TextureFormat};
use glam::UVec2;
use web_sys::{WebGlFramebuffer, WebGlRenderingContext as Gl};

pub struct Framebuffer {
    framebuffer: WebGlFramebuffer,
    texture: Texture,
}

impl Framebuffer {
    pub(crate) fn new(renderer: &Renderer, linear_filter: bool) -> Self {
        Self::with_viewport(&renderer.gl, renderer.canvas_size(), linear_filter)
    }

    pub(crate) fn with_viewport(gl: &Gl, viewport: UVec2, linear_filter: bool) -> Self {
        let mut texture = Texture::new_empty(gl, TextureFormat::Rgba, linear_filter);

        texture.realloc_with_opt_bytes(gl, viewport, None);

        // Create framebuffer and bind texture to it.
        let framebuffer = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&framebuffer));
        gl.framebuffer_texture_2d(
            Gl::FRAMEBUFFER,
            Gl::COLOR_ATTACHMENT0,
            Gl::TEXTURE_2D,
            Some(texture.get_inner()),
            0,
        );
        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);

        Self {
            framebuffer,
            texture,
        }
    }

    pub(crate) fn set_viewport(&mut self, gl: &Gl, viewport: UVec2) {
        if viewport != self.texture.dimensions {
            self.texture.realloc_with_opt_bytes(gl, viewport, None);
        }
    }

    pub(crate) fn bind<'a>(&'a mut self, renderer: &'a Renderer) -> FramebufferBinding<'a> {
        FramebufferBinding::new(renderer, self)
    }

    pub fn viewport(&self) -> UVec2 {
        self.texture.dimensions
    }

    pub fn as_texture(&self) -> &Texture {
        &self.texture
    }
}

pub struct FramebufferBinding<'a> {
    renderer: &'a Renderer,
    framebuffer: &'a Framebuffer,
}

impl<'a> FramebufferBinding<'a> {
    fn new(renderer: &'a Renderer, framebuffer: &'a Framebuffer) -> Self {
        // Set viewport and bind framebuffer.
        let gl = &renderer.gl;
        set_viewport(gl, framebuffer.viewport());
        gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&framebuffer.framebuffer));

        Self {
            renderer,
            framebuffer,
        }
    }

    pub fn viewport(&self) -> UVec2 {
        self.framebuffer.viewport()
    }
}

impl<'a> Drop for FramebufferBinding<'a> {
    fn drop(&mut self) {
        // Reset viewport and unbind framebuffer.
        let gl = &self.renderer.gl;
        set_viewport(gl, self.renderer.canvas_size());
        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
    }
}

fn set_viewport(gl: &Gl, viewport: UVec2) {
    let size = viewport.as_ivec2();
    gl.viewport(0, 0, size.x, size.y);
}
