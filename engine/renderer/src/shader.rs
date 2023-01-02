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

struct CachedUniform {
    location: Option<WebGlUniformLocation>,
    texture_index: usize,
}

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
    uniform_cache: RefCell<LinearMap<&'static str, CachedUniform>>,
    texture_index_alloc: Cell<usize>,
}

impl Shader {
    /// Compiles a new glsl shader from sources. Attribute locations are indexed exactly according
    /// to their index in the input.
    pub fn new(renderer: &Renderer, vertex: &str, fragment: &str) -> Self {
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
            texture_index_alloc: Cell::new(1), // 0 is reserved for creating textures.
        }))
    }

    /// Binds the [`Shader`] for handling subsequent draw calls. Returns `None` if the [`Shader`] is
    /// still compiling asynchronously.
    #[must_use]
    pub fn bind<'a>(&'a self, renderer: &'a Renderer) -> Option<ShaderBinding<'a>> {
        let khr = renderer.khr.as_ref();
        if !self.0.link_done.get() {
            if self
                .0
                .query_link_status(&renderer.gl, khr)
                .unwrap_or_else(|e| panic!("{}", e))
            {
                self.0.link_done.set(true);
            } else {
                return None;
            }
        }
        Some(ShaderBinding::new(renderer, &self.0))
    }
}

impl ShaderInner {
    fn location_inner<'a>(
        &'a self,
        gl: &Gl,
        name: &'static str,
        texture: bool,
    ) -> RefMut<'a, CachedUniform> {
        // Pre-borrow because using self in closure borrows all of self.
        let program = &self.program;
        let r = self.uniform_cache.borrow_mut();

        // Map mutable ref to avoid indexing again.
        RefMut::map(r, |r| {
            r.entry(name).or_insert_with(|| {
                let location = gl.get_uniform_location(program, name);
                let texture_index = if location.is_some() {
                    if texture {
                        let i = self.texture_index_alloc.get();
                        self.texture_index_alloc.set(i + 1);
                        i
                    } else {
                        usize::MAX // Set to invalid (not a texture).
                    }
                } else {
                    if cfg!(debug_assertions) {
                        console_log!("warning: uniform {} does not exist or is not in use", name);
                    }
                    usize::MAX // Set to invalid (doesn't exist or not in use).
                };

                CachedUniform {
                    location,
                    texture_index,
                }
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
                    e = prefix.to_string() + e.trim_end_matches('\x00');
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

/// A type which can be set as a uniform in a [`ShaderBinding`].
pub trait Uniform {
    #[doc(hidden)]
    fn uniform(self, shader: &ShaderBinding, name: &'static str);
}

impl Uniform for &Texture {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.shader.location_inner(shader.gl, name, true);
        if u.location.is_none() {
            return; // Unused or doesn't exist.
        }

        let index = u.texture_index;
        assert!(u.texture_index < 32, "using too many textures");

        shader.gl.uniform1i(u.location.as_ref(), index as i32);
        let mask = 1u32 << index;

        // Already bound unbind it.
        if shader.bound_textures.get() & mask != 0 {
            let cube = shader.bound_texture_cubes.get() & mask != 0;
            TextureBinding::drop_raw_parts(shader.renderer, index, cube);
        }

        // Use a bitset instead of a Vec<TextureBinding>.
        mem::forget(self.bind(shader.renderer, index));

        // Instead set into bitset.
        shader
            .bound_textures
            .set(shader.bound_textures.get() | mask);
        if self.typ().cube() {
            shader
                .bound_texture_cubes
                .set(shader.bound_texture_cubes.get() | mask);
        }
    }
}

impl Uniform for f32 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader.gl.uniform1f(u.as_ref(), self);
    }
}

impl Uniform for Vec2 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader.gl.uniform2f(u.as_ref(), self.x, self.y);
    }
}

impl Uniform for Vec3 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader.gl.uniform3f(u.as_ref(), self.x, self.y, self.z);
    }
}

impl Uniform for Vec4 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform4f(u.as_ref(), self.x, self.y, self.z, self.w);
    }
}

impl Uniform for &Mat2 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform_matrix2fv_with_f32_array(u.as_ref(), false, &self.to_cols_array());
    }
}

impl Uniform for &Mat3 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform_matrix3fv_with_f32_array(u.as_ref(), false, &self.to_cols_array());
    }
}

impl Uniform for &Mat4 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform_matrix4fv_with_f32_array(u.as_ref(), false, &self.to_cols_array());
    }
}

impl Uniform for i32 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader.gl.uniform1i(u.as_ref(), self);
    }
}

impl Uniform for IVec2 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader.gl.uniform2i(u.as_ref(), self.x, self.y);
    }
}

impl Uniform for IVec3 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader.gl.uniform3i(u.as_ref(), self.x, self.y, self.z);
    }
}

impl Uniform for IVec4 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform4i(u.as_ref(), self.x, self.y, self.z, self.w);
    }
}

#[cfg(feature = "webgl2")]
impl Uniform for u32 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader.gl.uniform1ui(u.as_ref(), self);
    }
}

#[cfg(feature = "webgl2")]
impl Uniform for UVec2 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader.gl.uniform2ui(u.as_ref(), self.x, self.y);
    }
}

#[cfg(feature = "webgl2")]
impl Uniform for UVec3 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader.gl.uniform3ui(u.as_ref(), self.x, self.y, self.z);
    }
}

#[cfg(feature = "webgl2")]
impl Uniform for UVec4 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform4ui(u.as_ref(), self.x, self.y, self.z, self.w);
    }
}

impl Uniform for bool {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        (self as i32).uniform(shader, name);
    }
}

impl Uniform for BVec2 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        IVec2::select(self, IVec2::ONE, IVec2::ZERO).uniform(shader, name);
    }
}

impl Uniform for BVec3 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        IVec3::select(self, IVec3::ONE, IVec3::ZERO).uniform(shader, name);
    }
}

impl Uniform for BVec4 {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        IVec4::select(self, IVec4::ONE, IVec4::ZERO).uniform(shader, name);
    }
}

impl<const N: usize> Uniform for &[f32; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform1fv_with_f32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

impl<const N: usize> Uniform for &[Vec2; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform2fv_with_f32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

impl<const N: usize> Uniform for &[Vec3; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform3fv_with_f32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

impl<const N: usize> Uniform for &[Vec4; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform4fv_with_f32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

impl<const N: usize> Uniform for &[Mat2; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform_matrix2fv_with_f32_array(u.as_ref(), false, bytemuck::cast_slice(self));
    }
}

impl<const N: usize> Uniform for &[Mat3; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform_matrix3fv_with_f32_array(u.as_ref(), false, bytemuck::cast_slice(self));
    }
}

impl<const N: usize> Uniform for &[Mat4; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform_matrix4fv_with_f32_array(u.as_ref(), false, bytemuck::cast_slice(self));
    }
}

impl<const N: usize> Uniform for &[i32; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform1iv_with_i32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

impl<const N: usize> Uniform for &[IVec2; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform2iv_with_i32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

impl<const N: usize> Uniform for &[IVec3; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform3iv_with_i32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

impl<const N: usize> Uniform for &[IVec4; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform4iv_with_i32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

#[cfg(feature = "webgl2")]
impl<const N: usize> Uniform for &[u32; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform1uiv_with_u32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

#[cfg(feature = "webgl2")]
impl<const N: usize> Uniform for &[UVec2; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform2uiv_with_u32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

#[cfg(feature = "webgl2")]
impl<const N: usize> Uniform for &[UVec3; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform3uiv_with_u32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

#[cfg(feature = "webgl2")]
impl<const N: usize> Uniform for &[UVec4; N] {
    fn uniform(self, shader: &ShaderBinding, name: &'static str) {
        let u = shader.location(name);
        shader
            .gl
            .uniform4uiv_with_u32_array(u.as_ref(), bytemuck::cast_slice(self));
    }
}

/// A bound [`Shader`] that can you can draw with.
pub struct ShaderBinding<'a> {
    renderer: &'a Renderer,
    gl: &'a Gl, // Makes uniforms calls shorter.
    shader: &'a ShaderInner,
    bound_textures: Cell<u32>,      // bitset
    bound_texture_cubes: Cell<u32>, // bitset that is a subset of bound_textures.
}

impl<'a> ShaderBinding<'a> {
    fn new(renderer: &'a Renderer, shader: &'a ShaderInner) -> Self {
        let gl = &renderer.gl;

        // Make sure binding was cleared.
        debug_assert!(gl.get_parameter(Gl::CURRENT_PROGRAM).unwrap().is_null());

        gl.use_program(Some(&shader.program));
        Self {
            renderer,
            gl: &renderer.gl,
            shader,
            bound_textures: Cell::new(0),
            bound_texture_cubes: Cell::new(0),
        }
    }

    /// Gets the (cached) location of a named uniform. TODO take and assert size.
    fn location(&self, name: &'static str) -> RefMut<'a, Option<WebGlUniformLocation>> {
        RefMut::map(self.shader.location_inner(self.gl, name, false), |r| {
            &mut r.location
        })
    }

    /// Sets a [`Uniform`] named `name` to a `value`.
    pub fn uniform(&self, name: &'static str, value: impl Uniform) {
        value.uniform(self, name)
    }
}

impl<'a> Drop for ShaderBinding<'a> {
    fn drop(&mut self) {
        // Unbind (not required in release mode).
        #[cfg(debug_assertions)]
        self.gl.use_program(None);

        let mut bitset = self.bound_textures.get();
        let cubes = self.bound_texture_cubes.get();

        for index in 0..32 {
            // Break early if no more bits.
            if bitset == 0 {
                break;
            }

            let bit = 1u32 << index;
            if bitset & bit != 0 {
                // Clear bit (so we can return early if no more bits).
                bitset ^= bit;

                let cube = cubes & bit != 0;
                TextureBinding::drop_raw_parts(self.renderer, index, cube);
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
