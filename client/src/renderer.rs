// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::buffer::*;
use crate::settings::Settings;
use crate::shader::Shader;
use crate::texture::Texture;
use client_util::console_log;
use common::transform::Transform;
use glam::{vec2, Mat2, Mat3, Vec2, Vec4};
use serde::Serialize;
use sprite_sheet::UvSpriteSheet;
use std::mem;
use std::ops::{Mul, Range};
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, OesStandardDerivatives, OesVertexArrayObject, WebGlRenderingContext as Gl,
};

pub struct KhrParallelShaderCompile;
impl KhrParallelShaderCompile {
    pub const COMPLETION_STATUS_KHR: u32 = 37297;
}

pub struct Renderer {
    pub canvas: HtmlCanvasElement,
    pub gl: Gl,
    pub khr: Option<KhrParallelShaderCompile>,
    pub oes_vao: OesVertexArrayObject,
    pub particle_shader: Shader,
    pub sprite_sheet: UvSpriteSheet,
    pub view_matrix: Mat3,

    background_geometry: RenderBuffer<Vec2>,
    background_shader: Shader,
    graphic_buffer: RenderBuffer<PosColor>,
    graphic_mesh: MeshBuffer<PosColor>,
    graphic_shader: Shader,
    grass_texture: Texture,
    sand_texture: Texture,
    sprite_buffer: RenderBuffer<PosUvAlpha>,
    sprite_mesh: MeshBuffer<PosUvAlpha>,
    sprite_shader: Shader,
    sprite_texture: Texture,
    text_geometry: RenderBuffer<Vec2>,
    text_shader: Shader,
}

macro_rules! include_shader {
    ($gl: expr, $name:literal, $($attribute:literal),+) => {
        Shader::new(
            $gl,
            include_str!(concat!(concat!("../shaders/", $name), ".vert")),
            include_str!(concat!(concat!("../shaders/", $name), ".frag")),
            $name,
            &[$($attribute, )+],
        )
    }
}

impl Renderer {
    // Creates a new WebGl 1.0 render, attaching it to the canvas element with the id "canvas."
    pub fn new(settings: Settings, sprite_path: &str, sprite_sheet: UvSpriteSheet) -> Self {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement =
            canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct ContextOptions {
            alpha: bool,
            antialias: bool,
            premultiplied_alpha: bool,
        }

        let options = serde_wasm_bindgen::to_value(&ContextOptions {
            alpha: true,
            antialias: settings.antialias,
            premultiplied_alpha: true,
        })
        .unwrap();

        let gl = canvas
            .get_context_with_context_options("webgl", &options)
            .unwrap()
            .unwrap()
            .dyn_into::<Gl>()
            .unwrap();

        let khr = gl
            .get_extension("KHR_parallel_shader_compile")
            .unwrap()
            .map(|_| KhrParallelShaderCompile);

        let oes_vao = gl
            .get_extension("OES_vertex_array_object")
            .unwrap()
            .unwrap()
            .unchecked_into::<OesVertexArrayObject>();

        if settings.render_waves {
            let oes_standard_derivatives = gl
                .get_extension("OES_standard_derivatives")
                .unwrap()
                .unwrap()
                .unchecked_into::<OesStandardDerivatives>();

            // No need to access this from Rust later.
            mem::forget(oes_standard_derivatives);
        }

        // Create shaders.
        let sprite_shader = include_shader!(&gl, "sprite", "position", "uv", "alpha");
        let particle_shader =
            include_shader!(&gl, "particle", "position", "velocity", "color", "radius", "created");
        let graphic_shader = include_shader!(&gl, "graphic", "position", "color");
        let text_shader = include_shader!(&gl, "text", "position", "uv", "color");

        let background_frag_template = include_str!("../shaders/background.frag");
        let mut background_frag_source = String::with_capacity(background_frag_template.len() + 20);

        if settings.render_foam {
            background_frag_source += "#define FOAM 1.0\n";
        }
        if settings.render_waves {
            background_frag_source += "#define WAVES 1.0\n";
        }
        background_frag_source += background_frag_template;

        let background_shader = Shader::new(
            &gl,
            include_str!("../shaders/background.vert"),
            &background_frag_source,
            "background",
            &["position"],
        );

        let (sand_path, grass_path) = if !settings.render_terrain_textures {
            // TODO: Kludge, using the principle that if the texture never loads, the placeholder
            // color will be used.
            ("/dummy.png", "/dummy.png")
        } else {
            ("/sand.png", "/grass.png")
        };

        // Load textures.
        let sand_texture = Texture::load(&gl, sand_path, Some([213, 176, 107]), true);
        let grass_texture = Texture::load(&gl, grass_path, Some([71, 85, 45]), true);

        let sprite_texture = Texture::load(&gl, sprite_path, None, false);

        // Create buffers.
        let sprite_mesh = MeshBuffer::new();
        let sprite_buffer = RenderBuffer::new(&gl, &oes_vao);

        let graphic_mesh = MeshBuffer::new();
        let graphic_buffer = RenderBuffer::new(&gl, &oes_vao);

        let mut background_geometry = RenderBuffer::new(&gl, &oes_vao);
        background_geometry.buffer(
            &gl,
            &[
                vec2(-1.0, 1.0),
                vec2(1.0, 1.0),
                vec2(-1.0, -1.0),
                vec2(1.0, -1.0),
            ],
            &[2, 0, 1, 2, 1, 3],
        );

        let mut text_geometry = RenderBuffer::new(&gl, &oes_vao);
        text_geometry.buffer(
            &gl,
            &[
                vec2(-0.5, 0.5),
                vec2(0.5, 0.5),
                vec2(-0.5, -0.5),
                vec2(0.5, -0.5),
            ],
            &[2, 0, 1, 2, 1, 3],
        );

        gl.clear_color(0.0, 0.20784314, 0.45490196, 1.0);
        gl.enable(Gl::BLEND);

        // First argument is Gl::SRC_ALPHA if not premultiplied alpha, Gl::ONE if premultiplied(?).
        gl.blend_func(Gl::ONE, Gl::ONE_MINUS_SRC_ALPHA);

        Self {
            background_geometry,
            background_shader,
            canvas,
            gl,
            graphic_buffer,
            graphic_mesh,
            graphic_shader,
            grass_texture,
            khr,
            oes_vao,
            particle_shader,
            sand_texture,
            sprite_buffer,
            sprite_mesh,
            sprite_shader,
            sprite_sheet,
            sprite_texture,
            text_geometry,
            text_shader,
            view_matrix: Mat3::ZERO,
        }
    }

    /// check_webgl_error checks for reportable WebGl errors and reports them to the browser console.
    /// This function stalls the graphics pipeline. Avoid it unless debugging.
    #[allow(dead_code)]
    pub fn check_webgl_error(&mut self) {
        console_log!(
            "WebGL error: {}",
            match self.gl.get_error() {
                Gl::NO_ERROR => return,
                Gl::INVALID_ENUM => "INVALID_ENUM",
                Gl::INVALID_VALUE => "INVALID_VALUE",
                Gl::INVALID_OPERATION => "INVALID_OPERATION",
                Gl::INVALID_FRAMEBUFFER_OPERATION => "INVALID_FRAMEBUFFER_OPERATION",
                Gl::OUT_OF_MEMORY => "OUT_OF_MEMORY",
                Gl::CONTEXT_LOST_WEBGL => "CONTEXT_LOST_WEBGL",
                _ => "UNKNOWN",
            }
        );
    }

    /// Returns the aspect ratio (width/height) of the canvas.
    pub fn aspect(&self) -> f32 {
        let width = self.canvas.client_width();
        let height = self.canvas.client_height();
        width as f32 / height as f32
    }

    /// start starts the renderer changing the aspect ratio if necessary,
    /// clearing the screen, and setting a new view matrix.
    pub fn start(&mut self, camera: Vec2, zoom: f32) {
        let aspect = self.aspect();

        // This matrix is manually inverted.
        self.view_matrix =
            Mat3::from_scale(vec2(1.0, aspect) / zoom).mul_mat3(&Mat3::from_translation(-camera));

        self.gl.viewport(
            0,
            0,
            self.canvas.width() as i32,
            self.canvas.height() as i32,
        );
        self.gl.clear(Gl::COLOR_BUFFER_BIT);
    }

    /// finish currently does nothing.
    pub fn finish(&mut self) {}

    /// render_sprite adds a sprite to the drawing queue.
    /// Returns whether the current frame of the animation is past the end, if applicable.
    pub fn add_sprite(
        &mut self,
        sprite: &str,
        frame: Option<usize>,
        dimensions: Vec2,
        transform: Transform,
        alpha: f32,
    ) {
        let sprite = if let Some(frame) = frame {
            let animation = &self.sprite_sheet.animations.get(sprite).unwrap();
            &animation[frame]
        } else {
            &self.sprite_sheet.sprites.get(sprite).expect(sprite)
        };

        let uvs = &sprite.uvs;

        let matrix = Mat3::from_scale_angle_translation(
            vec2(dimensions.x, dimensions.x * sprite.aspect),
            transform.direction.to_radians(),
            transform.position,
        );

        let mut vertices = [
            PosUvAlpha {
                pos: vec2(-0.5, 0.5),
                uv: uvs[0],
                alpha,
            },
            PosUvAlpha {
                pos: vec2(0.5, 0.5),
                uv: uvs[1],
                alpha,
            },
            PosUvAlpha {
                pos: vec2(-0.5, -0.5),
                uv: uvs[2],
                alpha,
            },
            PosUvAlpha {
                pos: vec2(0.5, -0.5),
                uv: uvs[3],
                alpha,
            },
        ];

        for vertex in vertices.iter_mut() {
            vertex.pos = matrix.transform_point2(vertex.pos);
        }

        for v in vertices {
            self.sprite_mesh.push_vertex(v);
        }
    }

    /// add_triangle_graphic adds a transformed equilateral triangle to the graphics queue, pointing
    /// upward if angle is zero.
    pub fn add_triangle_graphic(&mut self, pos: Vec2, scale: Vec2, angle: f32, color: Vec4) {
        let idx = self.graphic_mesh.vertices.len();
        self.graphic_mesh.indices.extend_from_slice(&[
            idx as Index,
            idx as Index + 1,
            idx as Index + 2,
        ]);
        let rot = Mat2::from_angle(angle);
        for delta in [
            vec2(-0.5, -0.5),
            vec2(0.0, 0.25 * 3f32.sqrt()),
            vec2(0.5, -0.5),
        ] {
            self.graphic_mesh.push_vertex(PosColor {
                pos: pos + rot.mul_vec2(delta.mul(scale)),
                color,
            });
        }
    }

    /// add_rectangle_graphic adds a transformed square to the graphics queue.
    pub fn add_rectangle_graphic(&mut self, pos: Vec2, scale: Vec2, angle: f32, color: Vec4) {
        let idx = self.graphic_mesh.vertices.len();
        self.graphic_mesh.push_quad([
            idx as Index,
            idx as Index + 1,
            idx as Index + 2,
            idx as Index + 3,
        ]);
        let half_scale = scale * 0.5;
        let rot = Mat2::from_angle(angle);
        for (dx, dy) in [
            (-half_scale.x, half_scale.y),
            (half_scale.x, half_scale.y),
            (-half_scale.x, -half_scale.y),
            (half_scale.x, -half_scale.y),
        ] {
            self.graphic_mesh.push_vertex(PosColor {
                pos: pos + rot * vec2(dx, dy),
                color,
            });
        }
    }

    /// add_rectangle_graphic adds a line to the graphics queue.
    pub fn add_line_graphic(&mut self, start: Vec2, end: Vec2, thickness: f32, color: Vec4) {
        let diff = end - start;
        let angle = diff.y.atan2(diff.x);
        self.add_rectangle_graphic(
            start + diff * 0.5,
            vec2(diff.length(), thickness),
            angle,
            color,
        );
    }

    /// add_arc_graphic adds an arc to the graphics queue.
    pub fn add_arc_graphic(
        &mut self,
        center: Vec2,
        radius: f32,
        range: Range<f32>,
        thickness: f32,
        color: Vec4,
    ) {
        use std::array;

        assert!(radius > 0.0, "radius must be positive");

        let angle_span = range.end - range.start;
        if angle_span <= 0.0 {
            // Nothing to draw.
            return;
        }

        // Number of segments to approximate an arc.
        // The radius.sqrt() helps even out the quality surprisingly well.
        let mut segments =
            (10.0 * radius.sqrt() * angle_span * (1.0 / (std::f32::consts::PI * 2.0))) as i32;

        // Set maximum to prevent indices from overflowing.
        segments = segments.clamp(2, ((u16::MAX - 3) / 2) as i32);
        let segments = segments as u32;

        // Algorithm: Build a circle outline segment by segment, going counterclockwise. Vertices
        // are reused for maximum efficiency (except the original A and B which are duplicated at the
        // end).

        /*
           D -> +.   <- angle & mat
              -    .
            -        .
          -            .
        +   <-- C        .
         \                .
          +---------------+
          ^               ^
          |               |
          A (index)       B
         */

        let inner = radius - thickness * 0.5;
        let outer = radius + thickness * 0.5;

        let initial_a = vec2(inner, 0.0);
        let initial_b = vec2(outer, 0.0);
        let mat = Mat2::from_angle(range.start);
        let a = center + mat * initial_a;
        let b = center + mat * initial_b;

        let vertices = &mut self.graphic_mesh.vertices;
        // Calculate before extending vertices.
        let starting_index = vertices.len() as u32;

        let angle_per_segment = angle_span / segments as f32;

        // Use extend instead of loop to allow pre-allocation.
        vertices.extend(
            array::IntoIter::new([PosColor { pos: a, color }, PosColor { pos: b, color }]).chain(
                (1..=segments).into_iter().flat_map(|i| {
                    let angle = i as f32 * angle_per_segment + range.start;
                    let mat = Mat2::from_angle(angle);

                    let c = center + mat * initial_a;
                    let d = center + mat * initial_b;

                    array::IntoIter::new([PosColor { pos: c, color }, PosColor { pos: d, color }])
                }),
            ),
        );

        // Use extend instead of loop to allow pre-allocation.
        self.graphic_mesh
            .indices
            .extend((0..segments).into_iter().flat_map(|i| {
                let index = (starting_index + i * 2) as Index;
                // Triangles are [A, D, B] and [A, C, D].
                array::IntoIter::new([index, index + 3, index + 1, index, index + 2, index + 3])
            }));
    }

    /// add_circle_graphic adds a circle to the graphics queue.
    pub fn add_circle_graphic(&mut self, center: Vec2, radius: f32, thickness: f32, color: Vec4) {
        self.add_arc_graphic(
            center,
            radius,
            0.0..std::f32::consts::PI * 2.0,
            thickness,
            color,
        );
    }

    /// render_text immediately draws any number of text textures to the screen.
    /// Requires an iterator of position, scale, color, and texture.
    /// Scale refers to the scale on the vertical axis.
    pub fn render_text<'a, I: IntoIterator<Item = (Vec2, f32, Vec4, &'a Texture)>>(
        &mut self,
        iter: I,
    ) {
        if let Some(mut shader) = self.text_shader.bind(&self.gl, self.khr.as_ref()) {
            let buffer = self.text_geometry.bind(&self.gl, &self.oes_vao);

            for (pos, scale, color, texture) in iter {
                shader.uniform_texture("uSampler", &texture, 0);

                let mat = Mat3::from_scale_angle_translation(
                    vec2(scale / texture.aspect, scale),
                    0.0,
                    pos,
                );

                shader.uniform_matrix3f("uView", &self.view_matrix.mul(mat));
                shader.uniform4f("uColor", color);

                buffer.draw(Gl::TRIANGLES);
            }
        }
    }

    /// render_sprites immediately renders all sprites queued for drawing.
    pub fn render_sprites(&mut self) {
        if self.sprite_mesh.is_empty() {
            return;
        }

        if let Some(mut shader) = self.sprite_shader.bind(&self.gl, self.khr.as_ref()) {
            shader.uniform_texture("uSampler", &self.sprite_texture, 0);
            shader.uniform_matrix3f("uView", &self.view_matrix);

            self.sprite_mesh.push_default_quads();
            self.sprite_buffer.buffer_mesh(&self.gl, &self.sprite_mesh);

            let buffer = self.sprite_buffer.bind(&self.gl, &self.oes_vao);
            buffer.draw(Gl::TRIANGLES);
        }

        self.sprite_mesh.clear();
    }

    /// render_graphics immediately renders all graphics queued for drawing.
    pub fn render_graphics(&mut self) {
        if self.graphic_mesh.is_empty() {
            return;
        }

        if let Some(mut shader) = self.graphic_shader.bind(&self.gl, self.khr.as_ref()) {
            shader.uniform_matrix3f("uView", &self.view_matrix);

            self.graphic_buffer
                .buffer_mesh(&self.gl, &self.graphic_mesh);

            let buffer = self.graphic_buffer.bind(&self.gl, &self.oes_vao);
            buffer.draw(Gl::TRIANGLES);
        }

        self.graphic_mesh.clear()
    }

    /// render_background immediately renders the background, including terrain.
    pub fn render_background(
        &mut self,
        texture: &Texture,
        matrix: &Mat3,
        middle: Vec2,
        visual_radius: f32,
        visual_restriction: f32,
        world_radius: f32,
        time: f32,
    ) {
        if let Some(mut shader) = self.background_shader.bind(&self.gl, self.khr.as_ref()) {
            shader.uniform_texture("uSampler", &texture, 0);
            shader.uniform_texture("uSand", &self.sand_texture, 1);
            shader.uniform_texture("uGrass", &self.grass_texture, 2);

            shader.uniform_matrix3f("uView", &self.view_matrix.inverse()); // NOTE: Inverted.
            shader.uniform_matrix3f("uTexture", &matrix);
            shader.uniform1f("uTime", time);
            shader.uniform1f("uBorder", world_radius);
            shader.uniform1f("uVisual", visual_radius);
            shader.uniform2f("uMiddle", middle);
            shader.uniform1f("uRestrict", visual_restriction);

            let buffer = self.background_geometry.bind(&self.gl, &self.oes_vao);
            buffer.draw(Gl::TRIANGLES);
        }
    }
}

/// Vec2 stores a vertex with (only) a given position.
impl Vertex for Vec2 {
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        Vec2::bind_attrib(attribs);
    }
}

/// PosUvAlpha stores a vertex with (only) a given position, texture coordinate, and alpha.
#[repr(C)]
pub struct PosUvAlpha {
    pub pos: Vec2,
    pub uv: Vec2,
    pub alpha: f32,
}

impl Vertex for PosUvAlpha {
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        Vec2::bind_attrib(attribs);
        Vec2::bind_attrib(attribs);
        f32::bind_attrib(attribs);
    }
}

/// PosColor stores a vertex with (only) a given position and color.
#[repr(C)]
pub struct PosColor {
    pub pos: Vec2,
    // This is normally 16 byte aligned (breaking attribute size assertion), but not with glam's scalar-math feature enabled.
    pub color: Vec4,
}

impl Vertex for PosColor {
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        Vec2::bind_attrib(attribs);
        Vec4::bind_attrib(attribs);
    }
}
