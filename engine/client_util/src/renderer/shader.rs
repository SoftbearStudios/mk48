// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::renderer::KhrParallelShaderCompile as Khr;
use crate::renderer::texture::{Texture, TextureBinding};
use glam::*;
use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashMap;
use std::mem;
use web_sys::{WebGlProgram, WebGlRenderingContext as Gl, WebGlShader, WebGlUniformLocation};

/// References a WebGl shader.
pub struct Shader {
    program: WebGlProgram,
    vert_shader: WebGlShader,
    frag_shader: WebGlShader,
    link_done: Cell<bool>,
    uniform_cache: RefCell<HashMap<&'static str, Option<WebGlUniformLocation>>>,
}

impl Shader {
    /// new compiles a new glsl shader from sources. Attribute locations are indexed exactly according
    /// to their index in the input.
    pub(crate) fn new(gl: &Gl, vertex: &str, fragment: &str) -> Self {
        let vert_shader = Self::compile_shader(gl, Gl::VERTEX_SHADER, vertex);
        let frag_shader = Self::compile_shader(gl, Gl::FRAGMENT_SHADER, fragment);

        let attributes = Self::parse_attributes(vertex);

        // Defers failing to shader bind.
        let program = Self::link_program(gl, &vert_shader, &frag_shader, attributes);

        Self {
            program,
            vert_shader,
            frag_shader,
            link_done: Cell::new(false),
            uniform_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Parse attribute names from shader source.
    fn parse_attributes(vertex_source: &str) -> impl Iterator<Item = &str> {
        vertex_source
            .lines()
            .filter(|l| l.starts_with("attribute"))
            .map(|l| {
                l.split(' ')
                    .nth(2)
                    .expect("couldn't parse attribute")
                    .trim_end_matches(';')
            })
    }

    /// uniform gets the (cached) location of a named uniform.
    fn uniform<'a>(
        &'a self,
        gl: &Gl,
        name: &'static str,
    ) -> RefMut<'a, HashMap<&'static str, Option<WebGlUniformLocation>>> {
        // Pre-borrow because using self in closure borrows all of self.
        let program = &self.program;
        let mut r = self.uniform_cache.borrow_mut();

        // Borrow mut.
        r.entry(name).or_insert_with(|| {
            let uniform = gl.get_uniform_location(program, name);
            if uniform.is_none() {
                crate::console_log!("warning: uniform {} does not exist or is not in use", name);
            }
            uniform
        });

        // Re-borrow immut.
        r
    }

    /// Binds the shader for handling subsequent draw calls.
    /// Returns None if the shader is still compiling asynchronously.
    pub(crate) fn bind<'a>(&'a self, gl: &'a Gl, khr: Option<&Khr>) -> Option<ShaderBinding<'a>> {
        if !self.link_done.get() {
            if self.query_link_status(gl, khr).unwrap() {
                self.link_done.set(true);
            } else {
                return None;
            }
        }
        Some(ShaderBinding::new(gl, self))
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
        attributes: impl Iterator<Item = &'a str>,
    ) -> WebGlProgram {
        let program = gl.create_program().unwrap();

        gl.attach_shader(&program, vert_shader);
        gl.attach_shader(&program, frag_shader);

        for (i, attribute) in attributes.enumerate() {
            gl.bind_attrib_location(&program, i as u32, attribute);
        }

        gl.link_program(&program);
        program
    }

    /// Returns either Ok with a bool of if its done compiling or and Err with a compile error.
    fn query_link_status(&self, gl: &Gl, khr: Option<&Khr>) -> Result<bool, String> {
        // return Ok(false) if async compile not complete.
        if khr.is_some()
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
            let program_err = gl
                .get_program_info_log(&self.program)
                .unwrap_or_else(String::new);

            let vert_err = gl
                .get_shader_info_log(&self.vert_shader)
                .unwrap_or_else(String::new);

            let frag_err = gl
                .get_shader_info_log(&self.frag_shader)
                .unwrap_or_else(String::new);

            Err(format!(
                "link failed: {}, vs: {}, fs: {}",
                program_err, vert_err, frag_err
            ))
        }
    }
}

/// Represents binding a shader to draw with. Will take care of unbinding on drop, at least in
/// debug mode.
pub struct ShaderBinding<'a> {
    gl: &'a Gl,
    shader: &'a Shader,
    bound_textures: Cell<u32>, // bitset
}

impl<'a> ShaderBinding<'a> {
    fn new(gl: &'a Gl, shader: &'a Shader) -> Self {
        // Make sure binding was cleared.
        debug_assert!(gl.get_parameter(Gl::CURRENT_PROGRAM).unwrap().is_null());

        gl.use_program(Some(&shader.program));
        Self {
            gl,
            shader,
            bound_textures: Cell::new(0),
        }
    }

    /// Sets a
    pub fn uniform_texture(&self, name: &'static str, texture: &Texture, index: usize) {
        self.uniform1i(name, index as i32);

        let mask = 1u32 << index;

        // Already bound unbind it.
        if self.bound_textures.get() & mask != 0 {
            let binding = unsafe { TextureBinding::from_static(self.gl, index) };
            drop(binding)
        }

        // Can't keep borrow of gl.
        mem::forget(texture.bind(self.gl, index));

        // Instead set into bitset.
        self.bound_textures.set(self.bound_textures.get() | mask);
    }

    /// Sets a single integer uniform.
    fn uniform1i(&self, name: &'static str, v: i32) {
        let r = self.shader.uniform(self.gl, name);
        let u = r[name].as_ref();
        self.gl.uniform1i(u, v);
    }

    /// Sets a float uniform.
    pub fn uniform1f(&self, name: &'static str, v: f32) {
        let r = self.shader.uniform(self.gl, name);
        let u = r[name].as_ref();
        self.gl.uniform1f(u, v);
    }

    /// Sets a vec2 uniforms.
    pub fn uniform2f(&self, name: &'static str, v: Vec2) {
        let r = self.shader.uniform(self.gl, name);
        let u = r[name].as_ref();
        self.gl.uniform2f(u, v.x, v.y);
    }

    /// Sets a vec3 uniform.
    pub fn uniform3f(&self, name: &'static str, v: Vec3) {
        let r = self.shader.uniform(self.gl, name);
        let u = r[name].as_ref();
        self.gl.uniform3f(u, v.x, v.y, v.z);
    }

    /// Sets a vec4 uniform.
    pub fn uniform4f(&self, name: &'static str, v: Vec4) {
        let r = self.shader.uniform(self.gl, name);
        let u = r[name].as_ref();
        self.gl.uniform4f(u, v.x, v.y, v.z, v.w);
    }

    /// Sets a mat2 uniform.
    pub fn uniform_matrix2f(&self, name: &'static str, m: &Mat2) {
        let r = self.shader.uniform(self.gl, name);
        let u = r[name].as_ref();
        self.gl
            .uniform_matrix2fv_with_f32_array(u, false, &m.to_cols_array());
    }

    /// Sets a mat3 uniform.
    pub fn uniform_matrix3f(&self, name: &'static str, m: &Mat3) {
        let r = self.shader.uniform(self.gl, name);
        let u = r[name].as_ref();
        self.gl
            .uniform_matrix3fv_with_f32_array(u, false, &m.to_cols_array());
    }

    /// Sets a mat4 uniform.
    pub fn uniform_matrix4f(&mut self, name: &'static str, m: &Mat4) {
        let r = self.shader.uniform(self.gl, name);
        let u = r[name].as_ref();
        self.gl
            .uniform_matrix4fv_with_f32_array(u, false, &m.to_cols_array());
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

                let binding = unsafe { TextureBinding::from_static(self.gl, index) };
                drop(binding)
            }
        }
    }
}
