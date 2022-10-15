// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::gl::*;
use crate::renderer::Renderer;
use crate::texture::{Texture, TextureBinding};
use glam::*;
use js_hooks::console_log;
use linear_map::LinearMap;
use std::cell::{Cell, RefCell, RefMut};
use std::mem;
use std::rc::Rc;
use web_sys::{WebGlProgram, WebGlShader, WebGlUniformLocation};

/// References a glsl shader. As cheap to clone as an [`Rc`].
#[derive(Clone)]
pub struct Shader(Rc<ShaderInner>);

struct ShaderInner {
    program: WebGlProgram,
    vert_shader: WebGlShader,
    frag_shader: WebGlShader,
    link_done: Cell<bool>,
    // Use a LinearMap because there are relatively few uniforms.
    // TODO make a macro to convert uniform names to indices in a vec.
    uniform_cache: RefCell<LinearMap<&'static str, Option<WebGlUniformLocation>>>,
}

impl Shader {
    /// Compiles a new glsl shader from sources. Attribute locations are indexed exactly according
    /// to their index in the input.
    pub fn new<C>(renderer: &Renderer<C>, vertex: &str, fragment: &str) -> Self {
        let gl = &renderer.gl;
        let vert_shader = compile_shader(gl, Gl::VERTEX_SHADER, vertex);
        let frag_shader = compile_shader(gl, Gl::FRAGMENT_SHADER, fragment);

        // Defers failing to shader bind.
        let program = link_program(gl, &vert_shader, &frag_shader, parse_attributes(vertex));

        Self(Rc::new(ShaderInner {
            program,
            vert_shader,
            frag_shader,
            link_done: Default::default(),
            uniform_cache: Default::default(),
        }))
    }

    /// Binds the shader for handling subsequent draw calls.
    /// Returns None if the shader is still compiling asynchronously.
    pub fn bind<'a, C>(&'a self, renderer: &'a Renderer<C>) -> Option<ShaderBinding<'a>> {
        let gl = &renderer.gl;
        let khr = renderer.khr.as_ref();
        if !self.0.link_done.get() {
            if self
                .0
                .query_link_status(gl, khr)
                .unwrap_or_else(|e| panic!("{}", e))
            {
                self.0.link_done.set(true);
            } else {
                return None;
            }
        }
        Some(ShaderBinding::new(gl, &self.0))
    }
}

impl ShaderInner {
    /// uniform gets the (cached) location of a named uniform.
    fn uniform<'a>(
        &'a self,
        gl: &Gl,
        name: &'static str,
    ) -> RefMut<'a, Option<WebGlUniformLocation>> {
        // Pre-borrow because using self in closure borrows all of self.
        let program = &self.program;
        let r = self.uniform_cache.borrow_mut();

        // Map mutable ref to avoid indexing again.
        RefMut::map(r, |r| {
            r.entry(name).or_insert_with(|| {
                let uniform = gl.get_uniform_location(program, name);
                if uniform.is_none() && cfg!(debug_assertions) {
                    console_log!("warning: uniform {} does not exist or is not in use", name);
                }
                uniform
            })
        })
    }

    /// Returns either Ok with a bool of if its done compiling or and Err with a compile error.
    fn query_link_status(&self, gl: &Gl, khr: Option<&Khr>) -> Result<bool, String> {
        // return Ok(false) if async compile not complete.
        if !cfg!(feature = "blocking")
            && khr.is_some()
            && !gl
                .get_program_parameter(&self.program, Khr::COMPLETION_STATUS_KHR)
                .as_bool()
                .unwrap()
        {
            return Ok(false);
        }

        if gl
            .get_program_parameter(&self.program, Gl::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(true)
        } else {
            fn fmt_err(e: Option<String>, prefix: &str) -> String {
                let mut e = e.unwrap_or_default();
                if !e.is_empty() {
                    e = prefix.to_string() + e.trim_end_matches("\x00");
                }
                e
            }

            let mut error = fmt_err(gl.get_program_info_log(&self.program), "\n");
            error += &fmt_err(gl.get_shader_info_log(&self.vert_shader), "vs: ");
            error += &fmt_err(gl.get_shader_info_log(&self.frag_shader), "fs: ");
            Err(error)
        }
    }
}

/// A bound [`Shader`] that can you can draw with.
pub struct ShaderBinding<'a> {
    gl: &'a Gl,
    shader: &'a ShaderInner,
    bound_textures: Cell<u32>, // bitset
}

impl<'a> ShaderBinding<'a> {
    fn new(gl: &'a Gl, shader: &'a ShaderInner) -> Self {
        // Make sure binding was cleared.
        debug_assert!(gl.get_parameter(Gl::CURRENT_PROGRAM).unwrap().is_null());

        gl.use_program(Some(&shader.program));
        Self {
            gl,
            shader,
            bound_textures: Cell::new(0),
        }
    }

    /// Sets a `texture2D`/`sampler2D` uniform at an `index` in range `0..32`.
    pub fn uniform_texture(&self, name: &'static str, texture: &Texture, index: usize) {
        self.uniform1i(name, index as i32);

        let mask = 1u32 << index;

        // Already bound unbind it.
        if self.bound_textures.get() & mask != 0 {
            TextureBinding::drop_raw_parts(self.gl, index);
        }

        // Can't keep borrow of gl.
        mem::forget(texture.bind(self.gl, index));

        // Instead set into bitset.
        self.bound_textures.set(self.bound_textures.get() | mask);
    }

    /// Sets an `int` uniform.
    fn uniform1i(&self, name: &'static str, v: i32) {
        let u = self.shader.uniform(self.gl, name);
        self.gl.uniform1i(u.as_ref(), v);
    }

    /// Sets a `float` uniform.
    pub fn uniform1f(&self, name: &'static str, v: f32) {
        let u = self.shader.uniform(self.gl, name);
        self.gl.uniform1f(u.as_ref(), v);
    }

    /// Sets a `vec2` uniform.
    pub fn uniform2f(&self, name: &'static str, v: Vec2) {
        let u = self.shader.uniform(self.gl, name);
        self.gl.uniform2f(u.as_ref(), v.x, v.y);
    }

    /// Sets a `vec3` uniform.
    pub fn uniform3f(&self, name: &'static str, v: Vec3) {
        let u = self.shader.uniform(self.gl, name);
        self.gl.uniform3f(u.as_ref(), v.x, v.y, v.z);
    }

    /// Sets a `vec4` uniform.
    pub fn uniform4f(&self, name: &'static str, v: Vec4) {
        let u = self.shader.uniform(self.gl, name);
        self.gl.uniform4f(u.as_ref(), v.x, v.y, v.z, v.w);
    }

    /// Sets an array of `float` uniforms.
    pub fn uniform1fs(&self, name: &'static str, v: &[f32]) {
        let u = self.shader.uniform(self.gl, name);
        self.gl
            .uniform1fv_with_f32_array(u.as_ref(), bytemuck::cast_slice(v));
    }

    /// Sets an array of `vec3` uniforms.
    pub fn uniform2fs(&self, name: &'static str, v: &[Vec2]) {
        let u = self.shader.uniform(self.gl, name);
        self.gl
            .uniform2fv_with_f32_array(u.as_ref(), bytemuck::cast_slice(v));
    }

    /// Sets an array of `vec3` uniforms.
    pub fn uniform3fs(&self, name: &'static str, v: &[Vec3]) {
        let u = self.shader.uniform(self.gl, name);
        self.gl
            .uniform3fv_with_f32_array(u.as_ref(), bytemuck::cast_slice(v));
    }

    /// Sets an array of `vec4` uniforms.
    pub fn uniform4fs(&self, name: &'static str, v: &[Vec4]) {
        let u = self.shader.uniform(self.gl, name);
        self.gl
            .uniform4fv_with_f32_array(u.as_ref(), bytemuck::cast_slice(v));
    }

    /// Sets a `mat2` uniform.
    pub fn uniform_matrix2f(&self, name: &'static str, m: &Mat2) {
        let u = self.shader.uniform(self.gl, name);
        self.gl
            .uniform_matrix2fv_with_f32_array(u.as_ref(), false, &m.to_cols_array());
    }

    /// Sets a `mat3` uniform.
    pub fn uniform_matrix3f(&self, name: &'static str, m: &Mat3) {
        let u = self.shader.uniform(self.gl, name);
        self.gl
            .uniform_matrix3fv_with_f32_array(u.as_ref(), false, &m.to_cols_array());
    }

    /// Sets a `mat4` uniform.
    pub fn uniform_matrix4f(&self, name: &'static str, m: &Mat4) {
        let u = self.shader.uniform(self.gl, name);
        self.gl
            .uniform_matrix4fv_with_f32_array(u.as_ref(), false, &m.to_cols_array());
    }
}

impl<'a> Drop for ShaderBinding<'a> {
    fn drop(&mut self) {
        // Unbind (not required in release mode).
        #[cfg(debug_assertions)]
        self.gl.use_program(None);

        let mut bitset = self.bound_textures.get();
        for index in 0..32 {
            // Break early if no more bits.
            if bitset == 0 {
                break;
            }

            let bit = bitset & (1u32 << index);
            if bit != 0 {
                // Clear bit.
                bitset ^= bit;

                TextureBinding::drop_raw_parts(self.gl, index);
            }
        }
    }
}

/// Parse attribute names from shader source.
fn parse_attributes(vertex_source: &str) -> impl Iterator<Item = (&str, u32)> {
    #[allow(unused_mut)]
    let mut attribute = "attribute";
    #[cfg(feature = "webgl2")]
    if vertex_source.starts_with("#version 300 es") {
        // Won't match attributes like "layout(location = 0) in vec4 pos" which is actually
        // benificial because those don't require calling bind_attrib_location.
        attribute = "in";
    }

    debug_assert!(
        !vertex_source.contains("/*"),
        "attribute parser cannot handle multiline comments in vertex shader"
    );

    // TODO error/support attributes that don't start on new line.
    vertex_source.lines().filter_map(|l| {
        let mut tokens = l.split_ascii_whitespace();
        (tokens.next() == Some(attribute)).then(|| {
            let type_ = tokens.next().expect("attribute missing type");
            let name = tokens
                .next()
                .expect("attribute missing name")
                .trim_end_matches(';');

            let size = match type_ {
                "mat3" => 3,
                "mat4" => 4,
                _ => 1,
            };
            (name, size)
        })
    })
}

/// compile_shader combines either the vertex or fragment shader of a shader program.
fn compile_shader(gl: &Gl, shader_type: u32, source: &str) -> WebGlShader {
    let shader = gl.create_shader(shader_type).unwrap();
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    shader
}

/// link_program links the two shaders to form a shader program. It indexes attribute locations
/// in the exact order they appear in the input.
fn link_program<'a>(
    gl: &Gl,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
    attributes: impl Iterator<Item = (&'a str, u32)>,
) -> WebGlProgram {
    let program = gl.create_program().unwrap();

    gl.attach_shader(&program, vert_shader);
    gl.attach_shader(&program, frag_shader);

    let mut location = 0;
    for (name, size) in attributes {
        gl.bind_attrib_location(&program, location, name);
        location += size;
    }

    gl.link_program(&program);
    program
}
