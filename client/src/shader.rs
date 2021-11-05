// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::console_log;
use std::collections::HashMap;
use web_sys::{WebGlProgram, WebGlRenderingContext as Gl, WebGlShader, WebGlUniformLocation};

pub struct Shader {
    pub program: WebGlProgram,
    uniform_cache: HashMap<&'static str, Option<WebGlUniformLocation>>,
}

impl Shader {
    /// new compiles a new glsl shader from sources. Attribute locations are indexed exactly according
    /// to their index in the input.
    pub fn new(gl: &Gl, vertex: &str, fragment: &str, attributes: &[&'static str]) -> Self {
        let vert_shader = Self::compile_shader(gl, Gl::VERTEX_SHADER, vertex).unwrap();

        let frag_shader = Self::compile_shader(gl, Gl::FRAGMENT_SHADER, fragment).unwrap();

        let program = Self::link_program(gl, &vert_shader, &frag_shader, attributes).unwrap();

        Self {
            program,
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

    /// bind's the shader for handling subsequent draw calls.
    pub fn bind(&self, gl: &Gl) {
        gl.use_program(Some(&self.program));
    }

    /// Unbinds whatever shader is currently bound, if any.
    pub fn unbind(gl: &Gl) {
        gl.use_program(None);
    }

    /// compile_shader combines either the vertex or fragment shader of a shader program.
    fn compile_shader(context: &Gl, shader_type: u32, source: &str) -> Result<WebGlShader, String> {
        let shader = context
            .create_shader(shader_type)
            .expect("Unable to create shader object");
        context.shader_source(&shader, source);
        context.compile_shader(&shader);

        if context
            .get_shader_parameter(&shader, Gl::COMPILE_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(shader)
        } else {
            Err(context
                .get_shader_info_log(&shader)
                .expect("Unknown error creating shader"))
        }
    }

    /// link_program links the two shaders to form a shader program. It indexes attribute locations
    /// in the exact order they appear in the input.
    pub fn link_program(
        context: &Gl,
        vert_shader: &WebGlShader,
        frag_shader: &WebGlShader,
        attributes: &[&'static str],
    ) -> Result<WebGlProgram, String> {
        let program = context
            .create_program()
            .expect("Unable to create shader object");

        context.attach_shader(&program, vert_shader);
        context.attach_shader(&program, frag_shader);

        for (i, attribute) in attributes.iter().enumerate() {
            context.bind_attrib_location(&program, i as u32, attribute);
        }

        context.link_program(&program);

        if context
            .get_program_parameter(&program, Gl::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(program)
        } else {
            Err(context
                .get_program_info_log(&program)
                .unwrap_or_else(|| String::from("Unknown error creating program object")))
        }
    }
}
