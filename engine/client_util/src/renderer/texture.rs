// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::UVec2;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::CanvasRenderingContext2d;
use web_sys::WebGlRenderingContext as Gl;
use web_sys::WebGlTexture;

/// Texture references a WebGL texture. There are several options for creating one.
pub struct Texture {
    inner: Rc<WebGlTexture>,
    format: TextureFormat,
    pub dimensions: UVec2,
}

pub enum TextureFormat {
    Alpha,
    Rgba,
}

impl TextureFormat {
    /// Size of one pixel in bytes.
    fn pixel_size(&self) -> u32 {
        // If more options are added, make sure Self::pixel_align is kept up to date.
        match self {
            Self::Alpha => 1,
            Self::Rgba => 4,
        }
    }

    /// Alignment between pixels in bytes.
    fn pixel_align(&self) -> u32 {
        self.pixel_size()
    }

    /// Get the underlying WebGL format.
    fn as_format(&self) -> u32 {
        match self {
            Self::Alpha => Gl::ALPHA,
            Self::Rgba => Gl::RGBA,
        }
    }
}

impl Texture {
    /// Helper to calculate aspect ratio.
    fn from_inner(inner: Rc<WebGlTexture>, format: TextureFormat, dimensions: UVec2) -> Self {
        Self {
            inner,
            format,
            dimensions,
        }
    }

    pub(crate) fn get_inner(&self) -> &WebGlTexture {
        &self.inner
    }

    pub fn aspect(&self) -> f32 {
        let [width, height] = self.dimensions.as_vec2().to_array();
        width / height
    }

    /// Creates a new empty texture with the given formatting and fitler.
    /// Mipmaps and repeating cannot be used.
    pub(crate) fn new_empty(gl: &Gl, format: TextureFormat, linear_filter: bool) -> Self {
        let texture = Self::from_inner(Rc::new(gl.create_texture().unwrap()), format, UVec2::ZERO);
        gl.bind_texture(Gl::TEXTURE_2D, Some(&texture.inner));

        // Can't be repeating because size isn't known yet.
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);

        let filter = if linear_filter {
            Gl::LINEAR
        } else {
            Gl::NEAREST
        } as i32;

        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, filter);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, filter);

        unbind_texture_cfg_debug(gl);
        texture
    }

    /// Copies the bytes to the texture, resizing it if necessary.
    pub(crate) fn realloc_with_opt_bytes(
        &mut self,
        gl: &Gl,
        dimensions: UVec2,
        bytes: Option<&[u8]>,
    ) {
        gl.bind_texture(Gl::TEXTURE_2D, Some(&self.inner));

        // No mipmaps.
        let level = 0;
        let src_format = self.format.as_format();
        let src_type = Gl::UNSIGNED_BYTE;
        let [width, height] = dimensions.to_array();

        if let Some(bytes) = bytes {
            assert_eq!(
                width * height * self.format.pixel_size(),
                bytes.len() as u32
            );
        }

        // Set alignment if it's not the default.
        let align = self.format.pixel_align();
        if align != 4 {
            gl.pixel_storei(Gl::UNPACK_ALIGNMENT, 1);
        }

        // Don't reallocate if dimensions haven't changed.
        if self.dimensions == dimensions {
            gl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_u8_array(
                Gl::TEXTURE_2D,
                level,
                0,
                0,
                width as i32,
                height as i32,
                src_format,
                src_type,
                bytes,
            )
            .unwrap();
        } else {
            self.dimensions = dimensions;

            let internal_format = src_format;
            let border = 0;

            gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                Gl::TEXTURE_2D,
                level,
                internal_format as i32,
                width as i32,
                height as i32,
                border,
                src_format,
                src_type,
                bytes,
            )
            .unwrap();
        }

        // Reset to the default alignment.
        if align != 4 {
            gl.pixel_storei(Gl::UNPACK_ALIGNMENT, 4);
        }

        unbind_texture_cfg_debug(gl);
    }

    /// Creates a texture from some text, with variable length and constant height.
    /// Apply color to texture instead of shader so emoji colors are preserved.
    pub(crate) fn from_str_and_color(gl: &Gl, text: &str, color: [u8; 4]) -> Self {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.create_element("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ())
            .unwrap();

        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

        const FONT: &str = "30px Arial";
        const HEIGHT: u32 = 36; // 32 -> 36 to fit "😊".

        context.set_font(FONT);
        context.set_text_baseline("bottom");
        let text_width = context.measure_text(text).unwrap().width();

        let canvas_width = text_width as u32 + 2;
        canvas.set_width(canvas_width);
        canvas.set_height(HEIGHT);

        // Convert to css color (format hex, remove "0x" and prepend #).
        let mut color_hex = format!("{:#08x}", u32::from_be_bytes(color));
        color_hex = "#".to_owned() + color_hex.strip_prefix("0x").unwrap();

        context.set_fill_style(&JsValue::from_str(&color_hex));
        context.set_font(FONT);
        context.set_text_baseline("bottom");

        context
            .fill_text(text, 1.0, (HEIGHT - 1) as f64)
            .expect("could not fill text on canvas");

        let texture = Rc::new(gl.create_texture().unwrap());
        gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));
        gl.pixel_storei(Gl::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1);

        // No mipmaps since not always a power of 2.
        let level = 0;

        // Always use RGBA because text can have colored unicode.
        let format = TextureFormat::Rgba;
        let internal_format = format.as_format();
        let src_format = internal_format;
        let src_type = Gl::UNSIGNED_BYTE;

        gl.tex_image_2d_with_u32_and_u32_and_canvas(
            Gl::TEXTURE_2D,
            level,
            internal_format as i32,
            src_format,
            src_type,
            &canvas,
        )
        .expect("could not draw canvas to texture");

        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);

        gl.pixel_storei(Gl::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);

        unbind_texture_cfg_debug(gl);

        let dimensions = UVec2::new(canvas_width, HEIGHT);
        Self::from_inner(texture, format, dimensions)
    }

    /// Loads an RBGA texture from a URL.
    pub(crate) fn load(
        gl: &Gl,
        img_src: &str,
        dimensions: UVec2,
        placeholder: Option<[u8; 3]>,
        repeating: bool,
    ) -> Self {
        let texture = Rc::new(gl.create_texture().unwrap());
        gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));

        // Format is always RGBA for now.
        let format = TextureFormat::Rgba;
        let internal_format = format.as_format();
        let src_format = internal_format;
        let src_type = Gl::UNSIGNED_BYTE;

        // Unloaded textures are single pixel of placeholder or NONE.
        let level = 0;
        let width = 1;
        let height = 1;
        let border = 0;
        let p = placeholder.unwrap_or([0, 0, 0]);
        let pixel: [u8; 4] = [p[0], p[1], p[2], placeholder.is_some() as u8 * 255];

        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            Gl::TEXTURE_2D,
            level,
            internal_format as i32,
            width,
            height,
            border,
            src_format,
            src_type,
            Some(&pixel),
        )
        .unwrap();

        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(
            Gl::TEXTURE_2D,
            Gl::TEXTURE_MIN_FILTER,
            Gl::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);

        unbind_texture_cfg_debug(gl);

        let img = Rc::new(web_sys::HtmlImageElement::new().unwrap());

        // Callback when image is done loading.
        {
            let img2 = img.clone();
            let texture = texture.clone();
            let gl = Rc::new(gl.clone());
            let closure = Closure::wrap(Box::new(move || {
                gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));

                gl.pixel_storei(Gl::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1);

                if gl
                    .tex_image_2d_with_u32_and_u32_and_image(
                        Gl::TEXTURE_2D,
                        level,
                        internal_format as i32,
                        src_format,
                        src_type,
                        &img2,
                    )
                    .is_err()
                {
                    panic!("failed to load image");
                }

                let is_pow2 = img2.width().is_power_of_two() && img2.height().is_power_of_two();
                if is_pow2 {
                    gl.generate_mipmap(Gl::TEXTURE_2D);
                    gl.tex_parameteri(
                        Gl::TEXTURE_2D,
                        Gl::TEXTURE_MIN_FILTER,
                        Gl::LINEAR_MIPMAP_LINEAR as i32,
                    );
                } else {
                    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as i32);
                }

                gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);
                if repeating {
                    if is_pow2 {
                        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::REPEAT as i32);
                        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::REPEAT as i32);
                    } else {
                        panic!("repeating texture must be power of two")
                    }
                } else {
                    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
                    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
                }

                gl.pixel_storei(Gl::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);

                unbind_texture_cfg_debug(&gl);
            }) as Box<dyn FnMut()>);
            img.set_onload(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
        }

        // For compatibility with redirect scheme.
        img.set_cross_origin(Some("anonymous"));

        // Start loading image.
        img.set_src(img_src);

        Self::from_inner(texture, format, dimensions)
    }

    /// Bind a texture for affecting subsequent draw calls.
    pub(crate) fn bind<'a>(&self, gl: &'a Gl, index: usize) -> TextureBinding<'a> {
        TextureBinding::new(gl, index, self)
    }
}

#[allow(unused)]
pub struct TextureBinding<'a> {
    gl: &'a Gl,
    index: usize,
}

impl<'a> TextureBinding<'a> {
    fn new(gl: &'a Gl, index: usize, texture: &Texture) -> Self {
        active_texture(gl, index);

        // Make sure binding was cleared.
        debug_assert!(
            gl.get_parameter(Gl::TEXTURE_BINDING_2D).unwrap().is_null(),
            "texture already bound"
        );

        gl.bind_texture(Gl::TEXTURE_2D, Some(&texture.inner));
        Self { gl, index }
    }

    /// only for shader.rs
    /// It is unsafe because the TextureBinding must have been previously forgotten.
    pub(crate) unsafe fn from_static(gl: &'a Gl, index: usize) -> Self {
        Self { gl, index }
    }
}

impl<'a> Drop for TextureBinding<'a> {
    fn drop(&mut self) {
        // Unbind (not required in release mode).
        #[cfg(debug_assertions)]
        {
            active_texture(self.gl, self.index);
            unbind_texture(self.gl);
        }
    }
}

#[allow(unused)]
fn unbind_texture(gl: &Gl) {
    gl.bind_texture(Gl::TEXTURE_2D, None);
}

/// Unbinds the texture only in debug mode.
#[allow(unused)]
fn unbind_texture_cfg_debug(gl: &Gl) {
    #[cfg(debug_assertions)]
    unbind_texture(gl)
}

fn active_texture(gl: &Gl, index: usize) {
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Don't do redundant calls.
    static ACTIVE_TEXTURE: AtomicUsize = AtomicUsize::new(0);
    if index == ACTIVE_TEXTURE.load(Ordering::Relaxed) {
        return;
    }
    ACTIVE_TEXTURE.store(index, Ordering::Relaxed);

    assert!(index < 32, "only 32 textures supported");
    gl.active_texture(Gl::TEXTURE0 + index as u32);
}
