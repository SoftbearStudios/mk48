// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) use gl::*;

/// This module provides utilities to write code that is compatible with WebGL and WebGL2.
/// It acomplishes this by aliasing either WebGlRenderingContext or WebGl2RenderingContext to Gl and
/// aliasing WebGL extensions AngleInstancedArrays to Aia and OesVertexArrayObject to Ovao. The
/// extensions are replaced with WebGl2RenderingContext if it's enabled and compatibility traits are
/// provided to rename certain methods.

/// For compiling shaders in parallel. Uses the same api in WebGL/WebGL2.
pub(crate) struct KhrParallelShaderCompile;
impl KhrParallelShaderCompile {
    pub(crate) const COMPLETION_STATUS_KHR: u32 = 37297;
}
pub(crate) type Khr = KhrParallelShaderCompile;

// Doesn't have any fields so we don't need duplicates for WebGL/WebGL2.
#[cfg(feature = "srgb")]
pub(crate) type Srgb = web_sys::ExtSRgb;

#[cfg(feature = "anisotropy")]
pub(crate) type Ani = web_sys::ExtTextureFilterAnisotropic;

#[cfg(not(feature = "webgl2"))]
#[macro_use]
#[allow(clippy::module_inception)]
mod gl {
    use wasm_bindgen::JsCast;
    use web_sys::{AngleInstancedArrays, OesVertexArrayObject, WebGlRenderingContext};

    pub(crate) type Gl = WebGlRenderingContext;
    pub(crate) type Aia = AngleInstancedArrays;
    pub(crate) type Ovao = OesVertexArrayObject;

    /// Name of context for get_context call.
    pub(crate) const GL_NAME: &str = "webgl";

    // Use a macro so concat!() works.
    macro_rules! gl_title {
        () => {
            "WebGL"
        };
    }

    pub(crate) trait GlCompat {
        fn get_extension_aia(&self) -> Aia;
        fn get_extension_ovao(&self) -> Ovao;
    }

    impl GlCompat for Gl {
        fn get_extension_aia(&self) -> Aia {
            self.get_extension("ANGLE_instanced_arrays")
                .unwrap()
                .unwrap()
                .unchecked_into::<Aia>()
        }
        fn get_extension_ovao(&self) -> Ovao {
            self.get_extension("OES_vertex_array_object")
                .unwrap()
                .unwrap()
                .unchecked_into::<Ovao>()
        }
    }
}

#[cfg(feature = "webgl2")]
#[macro_use]
#[allow(clippy::module_inception)]
mod gl {
    use wasm_bindgen::JsValue;
    use web_sys::{
        HtmlCanvasElement, HtmlImageElement, WebGl2RenderingContext, WebGlVertexArrayObject,
    };

    pub(crate) type Gl = WebGl2RenderingContext;
    pub(crate) type Aia = Gl;
    pub(crate) type Ovao = Gl;

    pub(crate) const GL_NAME: &str = "webgl2";
    macro_rules! gl_title {
        () => {
            "WebGL2"
        };
    }

    pub(crate) trait GlCompat {
        fn tex_image_2d_with_u32_and_u32_and_canvas(
            &self,
            target: u32,
            level: i32,
            internalformat: i32,
            format: u32,
            type_: u32,
            source: &HtmlCanvasElement,
        ) -> Result<(), JsValue>;
        fn tex_image_2d_with_u32_and_u32_and_image(
            &self,
            target: u32,
            level: i32,
            internalformmat: i32,
            format: u32,
            type_: u32,
            source: &HtmlImageElement,
        ) -> Result<(), JsValue>;
        fn get_extension_ovao(&self) -> Ovao;
        fn get_extension_aia(&self) -> Aia;
    }

    impl GlCompat for Gl {
        fn tex_image_2d_with_u32_and_u32_and_canvas(
            &self,
            target: u32,
            level: i32,
            internalformat: i32,
            format: u32,
            type_: u32,
            source: &HtmlCanvasElement,
        ) -> Result<(), JsValue> {
            self.tex_image_2d_with_u32_and_u32_and_html_canvas_element(
                target,
                level,
                internalformat,
                format,
                type_,
                source,
            )
        }
        fn tex_image_2d_with_u32_and_u32_and_image(
            &self,
            target: u32,
            level: i32,
            internalformat: i32,
            format: u32,
            type_: u32,
            source: &HtmlImageElement,
        ) -> Result<(), JsValue> {
            self.tex_image_2d_with_u32_and_u32_and_html_image_element(
                target,
                level,
                internalformat,
                format,
                type_,
                source,
            )
        }
        fn get_extension_ovao(&self) -> Ovao {
            self.clone()
        }
        fn get_extension_aia(&self) -> Aia {
            self.clone()
        }
    }

    pub(crate) trait AiaCompat {
        const VERTEX_ATTRIB_ARRAY_DIVISOR_ANGLE: u32;
        fn draw_arrays_instanced_angle(&self, mode: u32, first: i32, count: i32, primcount: i32);
        fn draw_elements_instanced_angle_with_f64(
            &self,
            mode: u32,
            count: i32,
            type_: u32,
            offset: f64,
            primcount: i32,
        );
        fn draw_elements_instanced_angle_with_i32(
            &self,
            mode: u32,
            count: i32,
            type_: u32,
            offset: i32,
            primcount: i32,
        );
        fn vertex_attrib_divisor_angle(&self, index: u32, divisor: u32);
    }

    impl AiaCompat for Gl {
        const VERTEX_ATTRIB_ARRAY_DIVISOR_ANGLE: u32 = Self::VERTEX_ATTRIB_ARRAY_DIVISOR;
        fn draw_arrays_instanced_angle(&self, mode: u32, first: i32, count: i32, primcount: i32) {
            self.draw_arrays_instanced(mode, first, count, primcount)
        }
        fn draw_elements_instanced_angle_with_f64(
            &self,
            mode: u32,
            count: i32,
            type_: u32,
            offset: f64,
            primcount: i32,
        ) {
            self.draw_elements_instanced_with_f64(mode, count, type_, offset, primcount)
        }
        fn draw_elements_instanced_angle_with_i32(
            &self,
            mode: u32,
            count: i32,
            type_: u32,
            offset: i32,
            primcount: i32,
        ) {
            self.draw_elements_instanced_with_i32(mode, count, type_, offset, primcount)
        }
        fn vertex_attrib_divisor_angle(&self, index: u32, divisor: u32) {
            self.vertex_attrib_divisor(index, divisor)
        }
    }

    pub(crate) trait OvaoCompat {
        const VERTEX_ARRAY_BINDING_OES: u32;
        fn bind_vertex_array_oes(&self, array_object: Option<&WebGlVertexArrayObject>);
        fn create_vertex_array_oes(&self) -> Option<WebGlVertexArrayObject>;
        fn delete_vertex_array_oes(&self, array_object: Option<&WebGlVertexArrayObject>);
        fn is_vertex_array_oes(&self, array_object: Option<&WebGlVertexArrayObject>) -> bool;
    }

    impl OvaoCompat for Gl {
        const VERTEX_ARRAY_BINDING_OES: u32 = Self::VERTEX_ARRAY_BINDING;
        fn bind_vertex_array_oes(&self, array_object: Option<&WebGlVertexArrayObject>) {
            self.bind_vertex_array(array_object)
        }
        fn create_vertex_array_oes(&self) -> Option<WebGlVertexArrayObject> {
            self.create_vertex_array()
        }
        fn delete_vertex_array_oes(&self, array_object: Option<&WebGlVertexArrayObject>) {
            self.delete_vertex_array(array_object)
        }
        fn is_vertex_array_oes(&self, array_object: Option<&WebGlVertexArrayObject>) -> bool {
            self.is_vertex_array(array_object)
        }
    }
}
