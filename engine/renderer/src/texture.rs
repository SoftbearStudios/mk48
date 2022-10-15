// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::gl::*;
use crate::renderer::Renderer;
use crate::rgb::rgba_array_to_css;
use glam::UVec2;
use js_hooks::document;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::WebGlTexture;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Required for [`Texture::load`]'s callback.
struct TextureInner {
    texture: WebGlTexture,
    dimensions: Cell<UVec2>,
}

/// A 2d array of pixels that you can sample in a [`Shader`][`crate::shader::Shader`]. There
/// are several options for creating one. You can pass it as a uniform to a
/// [`ShaderBinding::uniform_texture`][`crate::shader::ShaderBinding::uniform_texture`].
pub struct Texture {
    inner: Rc<TextureInner>,
    format: TextureFormat,
}

/// A format of a [`Texture`]. Describes `bytes` in [`Texture::realloc_with_opt_bytes`] or the image
/// in [`Texture::load`].
#[derive(Copy, Clone)]
pub enum TextureFormat {
    /// 1 channel as alpha.
    Alpha,
    /// 3 channels as RGB.
    Rgb,
    /// 4 channels as RGBA.
    Rgba,
    /// 3 channels as sRGB.
    #[cfg(feature = "srgb")]
    Srgb,
    /// 4 channels as sRGB + alpha.
    #[cfg(feature = "srgb")]
    Srgba,
}

impl TextureFormat {
    /// Size of one pixel in bytes.
    fn pixel_size(&self) -> u32 {
        match self {
            Self::Alpha => 1,
            Self::Rgb => 3,
            Self::Rgba => 4,
            #[cfg(feature = "srgb")]
            Self::Srgb => 4,
            #[cfg(feature = "srgb")]
            Self::Srgba => 4,
        }
    }

    /// Alignment between pixels in bytes.
    fn pixel_align(&self) -> u32 {
        match self {
            Self::Alpha => 1,
            Self::Rgb => 1,
            Self::Rgba => 4,
            #[cfg(feature = "srgb")]
            Self::Srgb => 1,
            #[cfg(feature = "srgb")]
            Self::Srgba => 4,
        }
    }

    /// Get the underlying WebGL internal format.
    fn internal_format(&self) -> i32 {
        (match self {
            Self::Alpha => Gl::ALPHA,
            Self::Rgb => Gl::RGB,
            Self::Rgba => Gl::RGBA,
            #[cfg(all(not(feature = "webgl2"), feature = "srgb"))]
            Self::Srgb => Srgb::SRGB_EXT,
            #[cfg(all(feature = "webgl2", feature = "srgb"))]
            Self::Srgb => Gl::SRGB8,
            #[cfg(all(not(feature = "webgl2"), feature = "srgb"))]
            Self::Srgba => Srgb::SRGB_ALPHA_EXT,
            #[cfg(all(feature = "webgl2", feature = "srgb"))]
            Self::Srgba => Srgb::SRGB8_ALPHA8_EXT,
        }) as i32
    }

    /// Get the underlying WebGL src format.
    fn src_format(&self) -> u32 {
        #[cfg(not(feature = "webgl2"))]
        return self.internal_format() as u32;
        #[cfg(feature = "webgl2")]
        match self {
            Self::Alpha => Gl::ALPHA,
            Self::Rgb => Gl::RGB,
            Self::Rgba => Gl::RGBA,
            #[cfg(feature = "srgb")]
            Self::Srgb => Gl::RGB,
            #[cfg(feature = "srgb")]
            Self::Srgba => Gl::RGBA,
        }
    }

    /// Returns if a texture of this format can generate mipmaps. WebGL can't generate sRGB/sRGBA
    /// mipmaps. WebGL2 can generate sRGBA mipmaps but not sRGB ones for *some* reason.
    fn can_generate_mipmaps(&self) -> bool {
        match self {
            #[cfg(feature = "srgb")]
            Self::Srgb => false,
            #[cfg(feature = "srgb")]
            Self::Srgba => cfg!(feature = "webgl2"),
            _ => true,
        }
    }

    fn is_srgb(&self) -> bool {
        #[cfg(not(feature = "srgb"))]
        return false;
        #[cfg(feature = "srgb")]
        matches!(self, Self::Srgb | Self::Srgba)
    }

    fn has_alpha(&self) -> bool {
        matches!(self, Self::Alpha | Self::Rgba)
    }
}

impl Texture {
    pub(crate) fn new(gl: &Gl, dimensions: UVec2, format: TextureFormat) -> Self {
        Self {
            inner: Rc::new(TextureInner {
                texture: gl.create_texture().unwrap(),
                dimensions: Cell::new(dimensions),
            }),
            format,
        }
    }

    pub(crate) fn inner(&self) -> &WebGlTexture {
        &self.inner.texture
    }

    /// Gets aspect ratio (width / height).
    pub fn aspect(&self) -> f32 {
        let [width, height] = self.dimensions().as_vec2().to_array();
        width / height
    }

    /// Gets dimensions in pixels.
    pub fn dimensions(&self) -> UVec2 {
        self.inner.dimensions.get()
    }

    /// Creates a new empty [`Texture`] with the given `format` and `linear_filter`. Mipmaps and repeating
    /// cannot be used.
    pub fn new_empty<C>(
        renderer: &Renderer<C>,
        format: TextureFormat,
        linear_filter: bool,
    ) -> Self {
        let gl = &renderer.gl;
        let texture = Self::new(gl, UVec2::ZERO, format);
        gl.bind_texture(Gl::TEXTURE_2D, Some(texture.inner()));

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

    /// Copies the `bytes` to the [`Texture`], resizing to `dimensions` if necessary. The
    /// [`Texture`] must have been created with [`Texture::new_empty`].
    pub fn realloc_with_opt_bytes<C>(
        &mut self,
        renderer: &Renderer<C>,
        dimensions: UVec2,
        bytes: Option<&[u8]>,
    ) {
        let gl = &renderer.gl;
        gl.bind_texture(Gl::TEXTURE_2D, Some(self.inner()));

        // No mipmaps.
        let level = 0;
        let src_format = self.format.src_format();
        let src_type = Gl::UNSIGNED_BYTE;
        let [width, height] = dimensions.to_array();

        if let Some(bytes) = bytes {
            let pixel_size = self.format.pixel_size();
            assert_eq!(
                width * height * pixel_size,
                bytes.len() as u32,
                "{}x{}x{}",
                width,
                height,
                pixel_size
            );
        }

        // Set alignment if it's not the default.
        let align = self.format.pixel_align();
        if align != 4 {
            gl.pixel_storei(Gl::UNPACK_ALIGNMENT, 1);
        }

        // Don't reallocate if dimensions haven't changed.
        if self.dimensions() == dimensions {
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
            self.inner.dimensions.set(dimensions);

            let internal_format = self.format.internal_format();
            let border = 0;

            gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                Gl::TEXTURE_2D,
                level,
                internal_format,
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

    /// Creates a [`Texture`] from `text`, with variable length and constant height. Pass `color`
    /// to this function instead of coloring in a [`Shader`][`crate::shader::Shader`] so
    /// emoji colors are preserved.
    pub fn from_text<C>(renderer: &Renderer<C>, text: &str, color: [u8; 4]) -> Self {
        let (canvas, context) = create_canvas();

        const FONT: &str = "30px Arial";
        const HEIGHT: u32 = 36; // 32 -> 36 to fit "😊".

        context.set_font(FONT);
        context.set_text_baseline("bottom");
        let text_width = context.measure_text(text).unwrap().width();

        let canvas_width = text_width as u32 + 2;
        canvas.set_width(canvas_width);
        canvas.set_height(HEIGHT);

        let color_string = rgba_array_to_css(color);

        context.set_fill_style(&JsValue::from_str(&color_string));
        context.set_font(FONT);
        context.set_text_baseline("bottom");

        context
            .fill_text(text, 1.0, (HEIGHT - 1) as f64)
            .expect("could not fill text on canvas");

        let format = TextureFormat::Rgba;
        let dimensions = UVec2::new(canvas_width, HEIGHT);

        let gl = &renderer.gl;
        let texture = Self::new(gl, dimensions, format);
        gl.bind_texture(Gl::TEXTURE_2D, Some(texture.inner()));
        gl.pixel_storei(Gl::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1); // Canvas isn't premultiplied.

        // No mipmaps since not always a power of 2.
        let level = 0;

        // Always use RGBA because text can have colored unicode.
        let internal_format = format.internal_format();
        let src_format = format.src_format();
        let src_type = Gl::UNSIGNED_BYTE;

        gl.tex_image_2d_with_u32_and_u32_and_canvas(
            Gl::TEXTURE_2D,
            level,
            internal_format,
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
        texture
    }

    /// Loads an [`TextureFormat::Rgba`] [`Texture`] from `img_url`. You may specify a `placeholder`
    /// color for use before the image loads. You may also specify `repeating` if the loaded image
    /// has power of 2 dimensions.
    pub fn load<C>(
        renderer: &Renderer<C>,
        img_url: &str,
        format: TextureFormat,
        placeholder: Option<[u8; 3]>,
        repeating: bool,
    ) -> Self {
        assert!(!matches!(format, TextureFormat::Alpha), "not supported");

        let gl = &renderer.gl;
        let texture = Self::new(gl, UVec2::ONE, format);
        gl.bind_texture(Gl::TEXTURE_2D, Some(texture.inner()));

        let internal_format = format.internal_format();
        let src_format = format.src_format();
        let src_type = Gl::UNSIGNED_BYTE;

        // Unloaded textures are single pixel of placeholder or 0 alpha.
        let level = 0;
        let width = 1;
        let height = 1;
        let border = 0;

        let has_alpha = format.pixel_size() == 4;
        let owned_pixel;
        let pixel = if has_alpha {
            owned_pixel = placeholder.map(|p| [p[0], p[1], p[2], 255]);
            owned_pixel.as_ref().map(|p| p.as_slice())
        } else {
            placeholder.as_ref().map(|p| p.as_slice())
        };

        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            Gl::TEXTURE_2D,
            level,
            internal_format,
            width,
            height,
            border,
            src_format,
            src_type,
            pixel,
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
            let inner = texture.inner.clone();
            let gl = Rc::new(gl.clone());

            // Can't borrow renderer inside.
            #[cfg(feature = "anisotropy")]
            let anisotropy = renderer.anisotropy;

            let closure = Closure::wrap(Box::new(move || {
                gl.bind_texture(Gl::TEXTURE_2D, Some(&inner.texture));

                // Don't premultiply non srgb textures in srgb mode.
                let premultiply =
                    format.has_alpha() && (cfg!(not(feature = "srgb")) || format.is_srgb());
                if premultiply {
                    gl.pixel_storei(Gl::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1);
                }

                let dimensions = UVec2::new(img2.width(), img2.height());

                // Polyfill: clamp to max texture size to avoid errors. 2048 is minimum size
                // supported by all browsers.
                let max_dimensions = UVec2::splat(
                    gl.get_parameter(Gl::MAX_TEXTURE_SIZE)
                        .map(|v: JsValue| v.as_f64().unwrap_or_default() as u32)
                        .unwrap_or(0)
                        .max(2048),
                );
                let old_dim = dimensions;
                let dimensions = dimensions.min(max_dimensions);

                // Update texture dimensions.
                inner.dimensions.set(dimensions);

                // Resize with canvas if needed.
                if dimensions != old_dim {
                    let (canvas, context) = create_canvas();

                    canvas.set_width(dimensions.x);
                    canvas.set_height(dimensions.y);

                    context
                        .draw_image_with_html_image_element_and_dw_and_dh(
                            &img2,
                            0.0,
                            0.0,
                            dimensions.x as f64,
                            dimensions.y as f64,
                        )
                        .expect("failed to resize image");

                    gl.tex_image_2d_with_u32_and_u32_and_canvas(
                        Gl::TEXTURE_2D,
                        level,
                        internal_format,
                        src_format,
                        src_type,
                        &canvas,
                    )
                    .expect("failed to load resized image");
                } else {
                    gl.tex_image_2d_with_u32_and_u32_and_image(
                        Gl::TEXTURE_2D,
                        level,
                        internal_format,
                        src_format,
                        src_type,
                        &img2,
                    )
                    .expect("failed to load image");
                }

                let is_pow2_or_webgl2 = cfg!(feature = "webgl2")
                    || (dimensions.x.is_power_of_two() && dimensions.y.is_power_of_two());

                if is_pow2_or_webgl2 && format.can_generate_mipmaps() {
                    gl.generate_mipmap(Gl::TEXTURE_2D);
                    gl.tex_parameteri(
                        Gl::TEXTURE_2D,
                        Gl::TEXTURE_MIN_FILTER,
                        Gl::LINEAR_MIPMAP_LINEAR as i32,
                    );
                } else {
                    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as i32);
                }

                #[cfg(feature = "anisotropy")]
                if let Some(anisotropy) = anisotropy {
                    gl.tex_parameteri(
                        Gl::TEXTURE_2D,
                        Ani::TEXTURE_MAX_ANISOTROPY_EXT,
                        anisotropy as i32,
                    );
                }

                gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);
                if repeating {
                    if !is_pow2_or_webgl2 {
                        panic!("repeating texture must be power of two")
                    }
                    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::REPEAT as i32);
                    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::REPEAT as i32);
                } else {
                    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
                    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
                }

                if premultiply {
                    gl.pixel_storei(Gl::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);
                }

                unbind_texture_cfg_debug(&gl);
            }) as Box<dyn FnMut()>);
            img.set_onload(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
        }

        // For compatibility with redirect scheme.
        img.set_cross_origin(Some("anonymous"));

        // Start loading image.
        img.set_src(img_url);

        texture
    }

    /// Bind a texture for affecting subsequent draw calls.
    pub(crate) fn bind<'a>(&self, gl: &'a Gl, index: usize) -> TextureBinding<'a> {
        TextureBinding::new(gl, index, self)
    }
}

/// Creates a temporary canvas for drawing and then converting into a texture.
fn create_canvas() -> (HtmlCanvasElement, CanvasRenderingContext2d) {
    let canvas: HtmlCanvasElement = document()
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
        .unwrap();

    (canvas, context)
}

pub(crate) struct TextureBinding<'a> {
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

        gl.bind_texture(Gl::TEXTURE_2D, Some(texture.inner()));
        Self { gl, index }
    }

    /// Texture must have been created from the same index and passed to [`std::mem::forget`].
    pub(crate) fn drop_raw_parts(gl: &'a Gl, index: usize) {
        drop(Self { gl, index })
    }
}

impl<'a> Drop for TextureBinding<'a> {
    fn drop(&mut self) {
        // Set active texture (not required in release mode because not unbinding).
        if cfg!(debug_assertions) {
            active_texture(self.gl, self.index);
            unbind_texture_cfg_debug(self.gl)
        }
    }
}

// Unbind texture in debug mode (not required in release mode).
fn unbind_texture_cfg_debug(gl: &Gl) {
    if cfg!(debug_assertions) {
        gl.bind_texture(Gl::TEXTURE_2D, None);
    }
}

static ACTIVE_TEXTURE: AtomicUsize = AtomicUsize::new(0);

/// Call if renderer is recreated.
pub(crate) fn reset_active_texture() {
    ACTIVE_TEXTURE.store(0, Ordering::Relaxed);
}

fn active_texture(gl: &Gl, index: usize) {
    // Don't do redundant calls.
    if index == ACTIVE_TEXTURE.load(Ordering::Relaxed) {
        return;
    }
    ACTIVE_TEXTURE.store(index, Ordering::Relaxed);

    assert!(index < 32, "only 32 textures supported");
    gl.active_texture(Gl::TEXTURE0 + index as u32);
}
