// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::CanvasRenderingContext2d;
use web_sys::WebGlRenderingContext as Gl;
use web_sys::WebGlTexture;

pub struct Texture {
    inner: Rc<WebGlTexture>,
    pub(crate) aspect: f32, // height / width. NAN if unknown.
}

impl Texture {
    /// Creates a single-color-channel texture from bytes.
    pub fn from_bytes(gl: &Gl, width: u32, height: u32, bytes: &[u8]) -> Self {
        let texture = Rc::new(gl.create_texture().unwrap());
        gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));
        let level = 0;
        let internal_format = Gl::ALPHA;
        let border = 0;
        let src_format = Gl::ALPHA;
        let src_type = Gl::UNSIGNED_BYTE;

        assert_eq!(width * height, bytes.len() as u32);

        gl.pixel_storei(Gl::UNPACK_ALIGNMENT, 1);

        // Unloaded textures are single pixel of magenta.
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            Gl::TEXTURE_2D,
            level,
            internal_format as i32,
            width as i32,
            height as i32,
            border,
            src_format,
            src_type,
            Some(bytes),
        )
        .unwrap();

        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);

        gl.pixel_storei(Gl::UNPACK_ALIGNMENT, 4);

        Self::unbind(&gl);

        Self {
            inner: texture,
            aspect: height as f32 / width as f32,
        }
    }

    /// Creates a texture from some text, with variable length and constant height.
    pub fn from_str(gl: &Gl, text: &str) -> Self {
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
        const HEIGHT: u32 = 32;

        context.set_font(FONT);
        context.set_text_baseline("bottom");
        let text_width = context.measure_text(text).unwrap().width();

        canvas.set_width(text_width as u32 + 2);
        canvas.set_height(HEIGHT);

        context.set_fill_style(&JsValue::from_str("white"));
        context.set_font(FONT);
        context.set_text_baseline("bottom");

        context
            .fill_text(text, 1.0, (HEIGHT - 1) as f64)
            .expect("could not fill text on canvas");

        let texture = Rc::new(gl.create_texture().unwrap());
        gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));
        gl.pixel_storei(Gl::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 1);

        let level = 0;
        let internal_format = Gl::RGBA;
        let src_format = Gl::RGBA;
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

        Self::unbind(&gl);

        Self {
            inner: texture,
            aspect: HEIGHT as f32 / text_width as f32,
        }
    }

    /// Loads an RBGA texture from a URL.
    pub fn load(gl: &Gl, img_src: &str) -> Self {
        let texture = Rc::new(gl.create_texture().unwrap());
        gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));
        let level = 0;
        let internal_format = Gl::RGBA;
        let width = 1;
        let height = 1;
        let border = 0;
        let src_format = Gl::RGBA;
        let src_type = Gl::UNSIGNED_BYTE;

        // Unloaded textures are single pixel of magenta.
        let pixel: [u8; 4] = [255, 0, 255, 255];
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

        Self::unbind(&gl);

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

                if img2.width().is_power_of_two() && img2.height().is_power_of_two() {
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
                gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
                gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);

                gl.pixel_storei(Gl::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);

                Self::unbind(&gl);
            }) as Box<dyn FnMut()>);
            img.set_onload(Some(closure.as_ref().unchecked_ref()));
            closure.forget();
        }

        // Start loading image.
        img.set_src(img_src);

        Self {
            inner: texture,
            aspect: f32::NAN,
        }
    }

    /// Bind a texture for affecting subsequent draw calls.
    pub fn bind(&self, gl: &Gl) {
        gl.bind_texture(Gl::TEXTURE_2D, Some(&self.inner));
    }

    /// Unbinds any currently bound texture.
    pub fn unbind(gl: &Gl) {
        gl.bind_texture(Gl::TEXTURE_2D, None);
    }
}
