// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::KhrParallelShaderCompile as Khr;
use crate::texture::*;
use client_util::console_log;
use glam::*;
use std::collections::HashMap;
use std::mem;
use web_sys::{WebGlProgram, WebGlRenderingContext as Gl, WebGlShader, WebGlUniformLocation};

pub struct Shader {
    program: WebGlProgram,
    vert_shader: WebGlShader,
    frag_shader: WebGlShader,
    name: &'static str,
    link_done: bool,
    uniform_cache: HashMap<&'static str, Option<WebGlUniformLocation>>,
}

impl Shader {
    /// new compiles a new glsl shader from sources. Attribute locations are indexed exactly according
    /// to their index in the input.
    pub fn new(
        gl: &Gl,
        vertex: &str,
        fragment: &str,
        name: &'static str,
        attributes: &[&'static str],
    ) -> Self {
        let vert_shader = Self::compile_shader(gl, Gl::VERTEX_SHADER, vertex);
        let frag_shader = Self::compile_shader(gl, Gl::FRAGMENT_SHADER, fragment);

        // Defers failing to shader bind.
        let program = Self::link_program(gl, &vert_shader, &frag_shader, attributes);

        Self {
            program,
            vert_shader,
            frag_shader,
            name,
            link_done: false,
            uniform_cache: HashMap::new(),
        }
    }

    /// uniform gets the (cached) location of a named uniform.
    pub fn uniform(&mut self, gl: &Gl, name: &'static str) -> Option<&WebGlUniformLocation> {
        // Pre-borrow because using self in closure borrows all of self.
        let program = &self.program;
        self.uniform_cache
            .entry(name)
            .or_insert_with(|| {
                let uniform = gl.get_uniform_location(program, name);
                if uniform.is_none() {
                    console_log!("warning: uniform {} does not exist or is not in use", name);
                }
                uniform
            })
            .as_ref()
    }

    /// Binds the shader for handling subsequent draw calls.
    /// Returns None if the shader is still compiling asynchronously.
    pub fn bind<'a>(&'a mut self, gl: &'a Gl, khr: Option<&Khr>) -> Option<ShaderBinding<'a>> {
        if !self.link_done {
            if self.query_link_status(gl, khr).unwrap() {
                self.link_done = true;
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
    fn link_program(
        gl: &Gl,
        vert_shader: &WebGlShader,
        frag_shader: &WebGlShader,
        attributes: &[&'static str],
    ) -> WebGlProgram {
        let program = gl.create_program().unwrap();

        gl.attach_shader(&program, vert_shader);
        gl.attach_shader(&program, frag_shader);

        for (i, attribute) in attributes.iter().enumerate() {
            gl.bind_attrib_location(&program, i as u32, attribute);
        }

        gl.link_program(&program);
        program
    }

    /// Returns either Ok with a bool of if its done compiling or and Err with a compile error.
    fn query_link_status(&self, gl: &Gl, khr: Option<&Khr>) -> Result<bool, String> {
        // return Ok(false) if async compile not complete.
        if let Some(_) = khr {
            if !gl
                .get_program_parameter(&self.program, Khr::COMPLETION_STATUS_KHR)
                .as_bool()
                .unwrap()
            {
                return Ok(false);
            }
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
                .unwrap_or_else(|| String::new());

            let vert_err = gl
                .get_shader_info_log(&self.vert_shader)
                .unwrap_or_else(|| String::new());

            let frag_err = gl
                .get_shader_info_log(&self.frag_shader)
                .unwrap_or_else(|| String::new());

            Err(format!(
                "{} link failed: {}, vs: {}, fs: {}",
                self.name, program_err, vert_err, frag_err
            ))
        }
    }
}

pub struct ShaderBinding<'a> {
    gl: &'a Gl,
    shader: &'a mut Shader,
    bound_textures: u32, // bitset
}

impl<'a> ShaderBinding<'a> {
    fn new(gl: &'a Gl, shader: &'a mut Shader) -> Self {
        // Make sure binding was cleared.
        debug_assert!(gl.get_parameter(Gl::CURRENT_PROGRAM).unwrap().is_null());

        gl.use_program(Some(&shader.program));
        Self {
            gl,
            shader,
            bound_textures: 0,
        }
    }

    #[allow(unused)]
    pub fn uniform_texture(&mut self, name: &'static str, texture: &Texture, index: usize) {
        self.uniform1i(name, index as i32);

        let mask = 1u32 << index;

        // Already bound unbind it.
        if self.bound_textures & mask != 0 {
            let binding = unsafe { TextureBinding::from_static(self.gl, index) };
            drop(binding)
        }

        // Can't keep borrow of gl.
        mem::forget(texture.bind(self.gl, index));

        // Instead set into bitset.
        self.bound_textures |= mask;
    }

    fn uniform1i(&mut self, name: &'static str, v: i32) {
        self.gl.uniform1i(self.shader.uniform(self.gl, name), v)
    }

    #[allow(unused)]
    pub fn uniform1f(&mut self, name: &'static str, v: f32) {
        self.gl.uniform1f(self.shader.uniform(self.gl, name), v)
    }

    #[allow(unused)]
    pub fn uniform2f(&mut self, name: &'static str, v: Vec2) {
        self.gl
            .uniform2f(self.shader.uniform(self.gl, name), v.x, v.y)
    }

    #[allow(unused)]
    pub fn uniform3f(&mut self, name: &'static str, v: Vec3) {
        self.gl
            .uniform3f(self.shader.uniform(self.gl, name), v.x, v.y, v.z)
    }

    #[allow(unused)]
    pub fn uniform4f(&mut self, name: &'static str, v: Vec4) {
        self.gl
            .uniform4f(self.shader.uniform(self.gl, name), v.x, v.y, v.z, v.w)
    }

    #[allow(unused)]
    pub fn uniform_matrix2f(&mut self, name: &'static str, m: &Mat2) {
        self.gl.uniform_matrix2fv_with_f32_array(
            self.shader.uniform(self.gl, name),
            false,
            &m.to_cols_array(),
        )
    }

    #[allow(unused)]
    pub fn uniform_matrix3f(&mut self, name: &'static str, m: &Mat3) {
        self.gl.uniform_matrix3fv_with_f32_array(
            self.shader.uniform(self.gl, name),
            false,
            &m.to_cols_array(),
        )
    }

    #[allow(unused)]
    pub fn uniform_matrix4f(&mut self, name: &'static str, m: &Mat4) {
        self.gl.uniform_matrix4fv_with_f32_array(
            self.shader.uniform(self.gl, name),
            false,
            &m.to_cols_array(),
        )
    }
}

impl<'a> Drop for ShaderBinding<'a> {
    fn drop(&mut self) {
        // Unbind (not required in release mode).
        #[cfg(debug_assertions)]
        self.gl.use_program(None);

        let mut bitset = self.bound_textures;
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
