// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::console_log;
use crate::shader::Shader;
use crate::texture::Texture;
use common::transform::Transform;
use glam::{Mat2, Mat3, Vec2, Vec4};
use serde::Serialize;
use sprite_sheet::UvSpriteSheet;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::{Mul, Range};
use std::slice;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, OesVertexArrayObject, WebGlBuffer, WebGlRenderingContext as Gl,
    WebGlVertexArrayObject,
};

pub struct Renderer {
    background_geometry: RenderBuffer<Pos>,
    background_shader: Shader,
    text_geometry: RenderBuffer<PosUvColor>,
    text_shader: Shader,
    canvas: HtmlCanvasElement,
    oes_vao: OesVertexArrayObject,
    pub gl: Gl,
    sprite_buffer: RenderBuffer<PosUvAlpha>,
    sprite_mesh: MeshBuffer<PosUvAlpha>,
    sprite_shader: Shader,
    particle_mesh: MeshBuffer<Particle>,
    particle_buffer: RenderBuffer<Particle>,
    particle_shader: Shader,
    graphic_mesh: MeshBuffer<PosColor>,
    graphic_buffer: RenderBuffer<PosColor>,
    graphic_shader: Shader,
    pub sprite_sheet: UvSpriteSheet,
    sprite_texture: Texture,
    view_matrix: Mat3,
}

impl Renderer {
    // Creates a new WebGl 1.0 render, attaching it to the canvas element with the id "canvas."
    pub fn new(sprite_path: &str, sprite_sheet: UvSpriteSheet) -> Self {
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
            antialias: true,
            premultiplied_alpha: true,
        })
        .unwrap();

        let gl = canvas
            .get_context_with_context_options("webgl", &options)
            .unwrap()
            .unwrap()
            .dyn_into::<Gl>()
            .unwrap();

        let oes_vao = gl
            .get_extension("OES_vertex_array_object")
            .unwrap()
            .unwrap()
            .unchecked_into::<OesVertexArrayObject>();

        let sprite_shader = Shader::new(
            &gl,
            include_str!("../shaders/sprite.vert"),
            include_str!("../shaders/sprite.frag"),
            vec!["position", "uv", "alpha"],
        );

        let particle_shader = Shader::new(
            &gl,
            include_str!("../shaders/particle.vert"),
            include_str!("../shaders/particle.frag"),
            vec!["position", "color", "created"],
        );

        let background_shader = Shader::new(
            &gl,
            include_str!("../shaders/background.vert"),
            include_str!("../shaders/background.frag"),
            vec!["position"],
        );

        let graphic_shader = Shader::new(
            &gl,
            include_str!("../shaders/graphic.vert"),
            include_str!("../shaders/graphic.frag"),
            vec!["position", "color"],
        );

        let text_shader = Shader::new(
            &gl,
            include_str!("../shaders/text.vert"),
            include_str!("../shaders/text.frag"),
            vec!["position", "uv", "color"],
        );

        let sprite_texture = Texture::load(&gl, sprite_path);

        let sprite_mesh = MeshBuffer::new();
        let sprite_buffer = RenderBuffer::new(&gl, &oes_vao);

        let particle_mesh = MeshBuffer::new();
        let particle_buffer = RenderBuffer::new(&gl, &oes_vao);

        let graphic_mesh = MeshBuffer::new();
        let graphic_buffer = RenderBuffer::new(&gl, &oes_vao);

        let mut background_geometry = RenderBuffer::new(&gl, &oes_vao);
        background_geometry.buffer(
            &gl,
            vertices_from_floats::<Pos>(&[-1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0]),
            &[2, 0, 1, 2, 1, 3],
        );

        let text_geometry = RenderBuffer::new(&gl, &oes_vao);

        gl.clear_color(0.0, 0.20784314, 0.45490196, 1.0);
        gl.enable(Gl::BLEND);

        // First argument is Gl::SRC_ALPHA if not premultiplied alpha, Gl::ONE if premultiplied(?).
        gl.blend_func(Gl::ONE, Gl::ONE_MINUS_SRC_ALPHA);

        Self {
            canvas,
            gl,
            oes_vao,
            sprite_mesh,
            sprite_buffer,
            sprite_shader,
            particle_mesh,
            particle_buffer,
            particle_shader,
            background_shader,
            background_geometry,
            text_geometry,
            graphic_mesh,
            graphic_buffer,
            graphic_shader,
            sprite_sheet,
            sprite_texture,
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
        let width = self.canvas.width();
        let height = self.canvas.height();
        width as f32 / height as f32
    }

    /// reset resets the renderer, by clearing all added meshes, changing the aspect ratio if necessary,
    /// clearing the screen, and setting a new view matrix.
    pub fn reset(&mut self, camera: Vec2, zoom: f32) {
        self.sprite_mesh.clear();
        self.particle_mesh.clear();
        self.graphic_mesh.clear();

        let width = self.canvas.width();
        let height = self.canvas.height();
        let aspect = width as f32 / height as f32;

        // This matrix is manually inverted.
        self.view_matrix = Mat3::from_scale(Vec2::new(1.0, aspect) / zoom)
            .mul_mat3(&Mat3::from_translation(-camera));

        self.gl.viewport(0, 0, width as i32, height as i32);
        self.gl.clear(Gl::COLOR_BUFFER_BIT);
    }

    /// render_sprite adds a sprite to the drawing queue.
    /// Returns whether the current frame of the animation is past the end, if applicable.
    pub fn render_sprite(
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
            &self.sprite_sheet.sprites.get(sprite).unwrap()
        };

        let uvs = &sprite.uvs;

        let matrix = Mat3::from_scale_angle_translation(
            Vec2::new(dimensions.x, dimensions.x * sprite.aspect),
            transform.direction.to_radians(),
            transform.position,
        );

        let mut vertices = [
            PosUvAlpha {
                pos: Vec2::new(-0.5, 0.5),
                uv: uvs[0],
                alpha,
            },
            PosUvAlpha {
                pos: Vec2::new(0.5, 0.5),
                uv: uvs[1],
                alpha,
            },
            PosUvAlpha {
                pos: Vec2::new(-0.5, -0.5),
                uv: uvs[2],
                alpha,
            },
            PosUvAlpha {
                pos: Vec2::new(0.5, -0.5),
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

    /// add_particle queues a particle for drawing.
    pub fn add_particle(&mut self, pos: Vec2, color: f32, time: f32) {
        self.particle_mesh
            .push_vertex(Particle { pos, color, time });
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
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.0, 0.25 * 3f32.sqrt()),
            Vec2::new(0.5, -0.5),
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
                pos: pos + rot * Vec2::new(dx, dy),
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
            Vec2::new(diff.length(), thickness),
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
        let angle_span = range.end - range.start;
        let segments = (radius.sqrt() * (40.0 * std::f32::consts::PI) / angle_span) as i32;
        if segments < 2 {
            // Nothing to draw, avoid corruption later on.
            return;
        }

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

        let vertices = &mut self.graphic_mesh.vertices;
        let indices = &mut self.graphic_mesh.indices;

        let inner = radius - thickness * 0.5;
        let outer = radius + thickness * 0.5;

        let initial_a = Vec2::new(inner, 0.0);
        let initial_b = Vec2::new(outer, 0.0);
        let mat = Mat2::from_angle(range.start);
        let a = center + mat * initial_a;
        let b = center + mat * initial_b;
        let mut index = vertices.len() as Index;
        vertices.push(PosColor { pos: a, color });
        vertices.push(PosColor { pos: b, color });
        for segment in 1..=segments {
            let angle = range.start + angle_span * segment as f32 / segments as f32;
            let mat = Mat2::from_angle(angle);

            let c = center + mat * initial_a;
            let d = center + mat * initial_b;

            vertices.push(PosColor { pos: c, color });
            vertices.push(PosColor { pos: d, color });

            // A, D, B
            indices.push(index);
            indices.push(index + 3);
            indices.push(index + 1);

            // A, C, D
            indices.push(index);
            indices.push(index + 2);
            indices.push(index + 3);

            index += 2;

            /*
            Implicitly,
            a = c;
            b = d;
             */
        }
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

    /// render_text immediately draws text to the screen.
    /// scale refers to the scale on the vertical axis.
    pub fn render_text(&mut self, pos: Vec2, scale: f32, color: Vec4, texture: &Texture) {
        self.gl.active_texture(Gl::TEXTURE0);
        texture.bind(&self.gl);

        self.text_shader.bind(&self.gl);
        self.gl
            .uniform1i(self.text_shader.uniform(&self.gl, "uSampler"), 0);
        self.gl.uniform_matrix3fv_with_f32_array(
            self.text_shader.uniform(&self.gl, "uView"),
            false,
            &self.view_matrix.to_cols_array(),
        );

        let mat =
            Mat3::from_scale_angle_translation(Vec2::new(scale / texture.aspect, scale), 0.0, pos);

        let verts: Vec<PosUvColor> = [
            PosUvColor {
                pos: Vec2::new(-0.5, 0.5),
                uv: Vec2::new(0.0, 0.0),
                color,
            },
            PosUvColor {
                pos: Vec2::new(0.5, 0.5),
                uv: Vec2::new(1.0, 0.0),
                color,
            },
            PosUvColor {
                pos: Vec2::new(-0.5, -0.5),
                uv: Vec2::new(0.0, 1.0),
                color,
            },
            PosUvColor {
                pos: Vec2::new(0.5, -0.5),
                uv: Vec2::new(1.0, 1.0),
                color,
            },
        ]
        .iter()
        .map(|vert| PosUvColor {
            pos: mat.transform_point2(vert.pos),
            uv: vert.uv,
            color: vert.color,
        })
        .collect();

        self.text_geometry
            .buffer(&self.gl, &verts, &[2, 0, 1, 2, 1, 3]);
        self.text_geometry.bind(&self.gl, &self.oes_vao);
        self.text_geometry.draw(&self.gl, Gl::TRIANGLES);

        render_buffer_unbind(&self.oes_vao);
        Texture::unbind(&self.gl);
        Shader::unbind(&self.gl);
    }

    /// render_sprites immediately renders all sprites queued for drawing.
    pub fn render_sprites(&mut self) {
        if self.sprite_mesh.is_empty() {
            return;
        }
        self.gl.active_texture(Gl::TEXTURE0);
        self.sprite_texture.bind(&self.gl);

        self.sprite_shader.bind(&self.gl);
        self.gl.uniform_matrix3fv_with_f32_array(
            self.sprite_shader.uniform(&self.gl, "uView"),
            false,
            &self.view_matrix.to_cols_array(),
        );
        self.gl
            .uniform1i(self.sprite_shader.uniform(&self.gl, "uSampler"), 0);

        self.sprite_mesh.push_default_quads();
        self.sprite_buffer.buffer(
            &self.gl,
            &self.sprite_mesh.vertices,
            &self.sprite_mesh.indices,
        );

        self.sprite_buffer.bind(&self.gl, &self.oes_vao);
        self.sprite_buffer.draw(&self.gl, Gl::TRIANGLES);

        render_buffer_unbind(&self.oes_vao);
        Texture::unbind(&self.gl);
        Shader::unbind(&self.gl);
    }

    /// render_particles immediately renders all particles queued for drawing.
    pub fn render_particles(&mut self, time: f32) {
        if self.particle_mesh.is_empty() {
            return;
        }
        self.particle_shader.bind(&self.gl);
        self.gl.uniform_matrix3fv_with_f32_array(
            self.particle_shader.uniform(&self.gl, "uView"),
            false,
            &self.view_matrix.to_cols_array(),
        );
        self.gl
            .uniform1f(self.particle_shader.uniform(&self.gl, "uTime"), time);

        self.particle_mesh.push_default_points();
        self.particle_buffer.buffer(
            &self.gl,
            &self.particle_mesh.vertices,
            &self.particle_mesh.indices,
        );

        self.particle_buffer.bind(&self.gl, &self.oes_vao);
        self.particle_buffer.draw(&self.gl, Gl::POINTS);

        render_buffer_unbind(&self.oes_vao);
        Shader::unbind(&self.gl);

        // Clear particles (as multiple batches may be drawn).
        self.particle_mesh.clear();
    }

    /// render_graphics immediately renders all graphics queued for drawing.
    pub fn render_graphics(&mut self) {
        if self.graphic_mesh.is_empty() {
            return;
        }
        self.graphic_shader.bind(&self.gl);
        self.gl.uniform_matrix3fv_with_f32_array(
            self.graphic_shader.uniform(&self.gl, "uView"),
            false,
            &self.view_matrix.to_cols_array(),
        );

        self.graphic_buffer.buffer(
            &self.gl,
            &self.graphic_mesh.vertices,
            &self.graphic_mesh.indices,
        );

        self.graphic_buffer.bind(&self.gl, &self.oes_vao);
        self.graphic_buffer.draw(&self.gl, Gl::TRIANGLES);

        render_buffer_unbind(&self.oes_vao);
        Shader::unbind(&self.gl);
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
        self.gl.active_texture(Gl::TEXTURE0);
        texture.bind(&self.gl);

        self.background_shader.bind(&self.gl);
        self.gl.uniform_matrix3fv_with_f32_array(
            self.background_shader.uniform(&self.gl, "uView"),
            false,
            &self.view_matrix.inverse().to_cols_array(), // NOTE: Inverted.
        );
        self.gl
            .uniform1i(self.background_shader.uniform(&self.gl, "uSampler"), 0);
        self.gl
            .uniform1f(self.background_shader.uniform(&self.gl, "uTime"), time);
        self.gl.uniform_matrix3fv_with_f32_array(
            self.background_shader.uniform(&self.gl, "uTexture"),
            false,
            &matrix.to_cols_array(),
        );
        self.gl.uniform1f(
            self.background_shader.uniform(&self.gl, "uBorder"),
            world_radius,
        );
        self.gl.uniform1f(
            self.background_shader.uniform(&self.gl, "uVisual"),
            visual_radius,
        );
        self.gl.uniform2fv_with_f32_array(
            self.background_shader.uniform(&self.gl, "uMiddle"),
            middle.as_ref(),
        );
        self.gl.uniform1f(
            self.background_shader.uniform(&self.gl, "uRestrict"),
            visual_restriction,
        );

        self.background_geometry.bind(&self.gl, &self.oes_vao);
        self.background_geometry.draw(&self.gl, Gl::TRIANGLES);

        render_buffer_unbind(&self.oes_vao);
        Shader::unbind(&self.gl);
    }
}

/// Vertex is any vertex data consisting of floats.
trait Vertex: Sized {
    const FLOATS: usize;
    fn size() -> usize {
        Self::FLOATS * 4
    }
    fn bind_attribs(attribs: &mut Attribs<Self>);
}

struct Attribs<'a, V: Vertex> {
    gl: &'a Gl,
    bytes: u32,
    index: u32,
    vertex: PhantomData<V>,
}

impl<'a, V: Vertex> Attribs<'a, V> {
    fn new(gl: &'a Gl) -> Self {
        Self {
            gl,
            bytes: 0,
            index: 0,
            vertex: PhantomData,
        }
    }

    fn attrib(&mut self) -> u32 {
        let i = self.index;
        self.gl.enable_vertex_attrib_array(i);
        self.index += 1;
        i
    }

    fn offset(&mut self, bytes: usize) -> i32 {
        let b = self.bytes;
        self.bytes += bytes as u32;
        b as i32
    }

    fn floats(&mut self, count: usize) {
        self.gl.vertex_attrib_pointer_with_i32(
            self.attrib(),
            count as i32,
            Gl::FLOAT,
            false,
            V::size() as i32,
            self.offset(count * size_of::<f32>()),
        );
    }
}

impl<'a, V: Vertex> Drop for Attribs<'a, V> {
    fn drop(&mut self) {
        // Make sure all attributes were added.
        assert_eq!(self.bytes as usize, V::size());
        // Check safety of slice::from_raw_parts.
        assert_eq!(size_of::<V>(), V::size());
    }
}

/// Pos stores a vertex with (only) a given position.
#[repr(C)]
pub struct Pos {
    pub pos: Vec2,
}

impl Vertex for Pos {
    const FLOATS: usize = 2;
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        attribs.floats(2);
    }
}

/// PosUv stores a vertex with (only) a given position and texture coordinate.
#[repr(C)]
pub struct PosUv {
    pub pos: Vec2,
    pub uv: Vec2,
}

impl Vertex for PosUv {
    const FLOATS: usize = 4;
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        attribs.floats(2);
        attribs.floats(2);
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
    const FLOATS: usize = 5;
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        attribs.floats(2);
        attribs.floats(2);
        attribs.floats(1);
    }
}

/// PosUvColor stores a vertex with (only) a given position, texture coordinate, and color.
#[repr(C)]
pub struct PosUvColor {
    pub pos: Vec2,
    pub uv: Vec2,
    pub color: Vec4,
}

impl Vertex for PosUvColor {
    const FLOATS: usize = 8;
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        attribs.floats(2);
        attribs.floats(2);
        attribs.floats(4);
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
    const FLOATS: usize = 6;
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        attribs.floats(2);
        attribs.floats(4);
    }
}

/// Particle stores a vertex with (only) a given position, indexed color, and time (useful for particles).
#[repr(C)]
pub struct Particle {
    pub pos: Vec2,
    /// Possible values:
    /// -1 to 1: Fire to black
    ///  0 to 1: Black to white
    pub color: f32,
    pub time: f32,
}

impl Vertex for Particle {
    const FLOATS: usize = 4;
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        attribs.floats(2);
        attribs.floats(1);
        attribs.floats(1);
    }
}

type Index = u16;
type Quad = [Index; 4];

/// MeshBuffer allows building a mesh in RAM.
struct MeshBuffer<V: Vertex> {
    vertices: Vec<V>,
    indices: Vec<Index>,
}

impl<V: Vertex> MeshBuffer<V> {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    fn push_vertex(&mut self, vertex: V) {
        self.vertices.push(vertex);
    }

    fn push_quad(&mut self, quad: Quad) {
        self.indices
            .extend_from_slice(&[quad[0], quad[1], quad[2], quad[1], quad[3], quad[2]]);
    }

    fn push_default_quads(&mut self) {
        let n = self.vertices.len();
        assert_eq!(n % 4, 0);
        let quads = n / 4;

        for quad in 0..quads {
            let i = quad as Index * 4;
            self.push_quad([i, i + 1, i + 2, i + 3]);
        }
    }

    fn push_default_points(&mut self) {
        let n = self.vertices.len();
        for point in 0..n {
            self.indices.push(point as Index);
        }
    }

    fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}

/// RenderBuffer facilitates buffering a mesh to the GPU.
struct RenderBuffer<V: Vertex> {
    vertices: WebGlBuffer,
    indices: WebGlBuffer,
    vao: WebGlVertexArrayObject,
    index_count: u32,
    vertex_count: Index,
    vertex: PhantomData<V>,
}

impl<V: Vertex> RenderBuffer<V> {
    fn new(gl: &Gl, oes: &OesVertexArrayObject) -> Self {
        let buffer = Self {
            vertices: gl.create_buffer().unwrap(),
            indices: gl.create_buffer().unwrap(),
            vao: oes.create_vertex_array_oes().unwrap(),
            index_count: 0,
            vertex_count: 0,
            vertex: PhantomData,
        };

        // Bind buffers to vao.
        oes.bind_vertex_array_oes(Some(&buffer.vao));
        buffer.bind_without_vao(gl);
        oes.bind_vertex_array_oes(None);

        buffer
    }

    // bind_without_vao binds all the buffers to the vao.
    fn bind_without_vao(&self, gl: &Gl) {
        // Bind attributes.
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.vertices));
        V::bind_attribs(&mut Attribs::new(gl));

        // Bind index buffer.
        gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(&self.indices));
    }

    // bind must happen once before any number of draws.
    pub fn bind(&self, _gl: &Gl, oes: &OesVertexArrayObject) {
        oes.bind_vertex_array_oes(Some(&self.vao));
    }

    // draw draws the buffer.
    // It assumes that it is bound.
    fn draw(&self, gl: &Gl, primitive: u32) {
        gl.draw_elements_with_i32(primitive, self.index_count as i32, Gl::UNSIGNED_SHORT, 0);
    }

    // buffer moves the data from floats to the WebGL buffer.
    fn buffer(&mut self, gl: &Gl, vertices: &[V], indices: &[Index]) {
        self.index_count = indices.len() as u32;
        self.vertex_count = vertices.len() as Index;

        // Convert vertex slice to float slice.
        let floats = unsafe {
            let ptr = &vertices[0] as *const V as *const f32;
            let len = vertices.len() * V::FLOATS;
            slice::from_raw_parts(ptr, len)
        };

        // Buffer vertices.
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.vertices));
        unsafe {
            // Points to raw rust memory so can't allocate while in use.
            let vert_array = js_sys::Float32Array::view(floats);
            gl.buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &vert_array, Gl::STATIC_DRAW);
        }

        // Buffer indices.
        gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(&self.indices));
        unsafe {
            // Points to raw rust memory so can't allocate while in use.
            let elem_array = js_sys::Uint16Array::view(indices);
            gl.buffer_data_with_array_buffer_view(
                Gl::ELEMENT_ARRAY_BUFFER,
                &elem_array,
                Gl::STATIC_DRAW,
            );
        }
    }
}

/// render_buffer_unbind unbinds the currently bound RenderBuffer, if any.
pub fn render_buffer_unbind(oes: &OesVertexArrayObject) {
    oes.bind_vertex_array_oes(None);
}

/// vertices_from_floats reinterprets a slice of floats as a slice of vertices, panicking if the
/// given number of floats is not evenly divided by the vertex size.
fn vertices_from_floats<V: Vertex>(floats: &[f32]) -> &[V] {
    assert_eq!(floats.len() % V::FLOATS, 0);

    unsafe {
        let ptr = &floats[0] as *const f32 as *const V;
        let len = floats.len() / V::FLOATS;
        slice::from_raw_parts(ptr, len)
    }
}
