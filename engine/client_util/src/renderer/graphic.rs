// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::buffer::{MeshBuffer, RenderBuffer};
use crate::renderer::index::Index;
use crate::renderer::renderer::Layer;
use crate::renderer::renderer::Renderer;
use crate::renderer::shader::Shader;
use crate::renderer::vertex::PosColor;
use glam::{Mat2, Vec2, Vec4};
use std::array;
use std::cmp::Ordering;
use std::ops::Range;
use web_sys::WebGlRenderingContext as Gl;

/// Renders solid color polygons, known as graphics.
pub struct GraphicLayer<I: Index = u16> {
    mesh: MeshBuffer<PosColor, I>,
    buffer: RenderBuffer<PosColor, I>,
    /// Cached in pre_prepare.
    zoom: f32,
}

impl<I: Index> GraphicLayer<I> {
    pub fn new(renderer: &mut Renderer) -> Self {
        let gl = &renderer.gl;
        renderer.graphic_shader.get_or_insert_with(|| {
            Shader::new(
                gl,
                include_str!("./shaders/graphic.vert"),
                include_str!("./shaders/graphic.frag"),
            )
        });

        Self {
            mesh: MeshBuffer::new(),
            buffer: RenderBuffer::new(&renderer.gl, &renderer.oes_vao),
            zoom: 0.0,
        }
    }

    /// Adds arbitrary triangle(s), starting at index 0.
    pub fn add_triangles<VI: IntoIterator<Item = PosColor>, II: IntoIterator<Item = u16>>(
        &mut self,
        vertices: VI,
        indices: II,
    ) {
        let start = self.mesh.vertices.len();
        self.mesh.vertices.extend(vertices);
        self.mesh.indices.extend(
            indices
                .into_iter()
                .map(|idx| I::from_usize(start + idx as usize)),
        );
    }

    /// add_triangle_graphic adds a transformed equilateral triangle to the graphics queue, pointing
    /// upward if angle is zero.
    pub fn add_triangle(&mut self, center: Vec2, scale: Vec2, angle: f32, color: Vec4) {
        let index = self.mesh.vertices.len();
        self.mesh.indices.extend_from_slice(&[
            I::from_usize(index),
            I::from_usize(index + 1),
            I::from_usize(index + 2),
        ]);

        let rot = Mat2::from_angle(angle);
        let positions = [
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.0, 0.25 * 3f32.sqrt()),
            Vec2::new(0.5, -0.5),
        ];

        self.mesh.vertices.extend(positions.map(|pos| PosColor {
            pos: center + rot * (pos * scale),
            color,
        }));
    }

    /// add_rectangle_graphic adds a transformed square to the graphics queue.
    pub fn add_rectangle(&mut self, center: Vec2, scale: Vec2, angle: f32, color: Vec4) {
        self.add_rectangle_gradient(center, scale, angle, [color; 4])
    }

    pub fn add_rectangle_gradient(
        &mut self,
        center: Vec2,
        scale: Vec2,
        angle: f32,
        colors: [Vec4; 4],
    ) {
        let index = self.mesh.vertices.len();
        self.mesh.push_quad([
            Index::from_usize(index),
            Index::from_usize(index + 1),
            Index::from_usize(index + 2),
            Index::from_usize(index + 3),
        ]);

        let half_scale = scale * 0.5;
        let rot = Mat2::from_angle(angle);
        let positions = [
            Vec2::new(-half_scale.x, half_scale.y),
            Vec2::new(half_scale.x, half_scale.y),
            Vec2::new(-half_scale.x, -half_scale.y),
            Vec2::new(half_scale.x, -half_scale.y),
        ];

        self.mesh
            .vertices
            .extend(positions.zip(colors).map(|(pos, color)| PosColor {
                pos: center + rot * pos,
                color,
            }));
    }

    /// add_rectangle_graphic adds a line to the graphics queue.
    pub fn add_line(&mut self, start: Vec2, end: Vec2, thickness: f32, color: Vec4) {
        self.add_line_gradient(start, end, thickness, color, color)
    }

    pub fn add_line_gradient(&mut self, start: Vec2, end: Vec2, thickness: f32, s: Vec4, e: Vec4) {
        let diff = end - start;
        let angle = diff.y.atan2(diff.x);

        self.add_rectangle_gradient(
            start + diff * 0.5,
            Vec2::new(diff.length(), thickness),
            angle,
            [s, e, s, e],
        );
    }

    /// add_arc_graphic adds an arc to the graphics queue. Angles are in radians.
    pub fn add_arc(
        &mut self,
        center: Vec2,
        radius: f32,
        angle_range: Range<f32>,
        thickness: f32,
        color: Vec4,
    ) {
        match radius.partial_cmp(&0.0).unwrap() {
            Ordering::Greater => {}
            Ordering::Equal => return,
            Ordering::Less => panic!("radius can't be negative"),
        }

        let angle_span = angle_range.end - angle_range.start;
        if angle_span <= 0.0 {
            // Nothing to draw.
            return;
        }

        // Number of segments to approximate an arc.
        // The radius.sqrt() helps even out the quality surprisingly well.
        let segments = ((radius / self.zoom).sqrt()
            * angle_span
            * (200.0 / (std::f32::consts::PI * 2.0))) as i32;

        // Set maximum to prevent indices from overflowing.
        let segments = segments.clamp(6, 100) as usize;

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

        let initial_a = Vec2::new(inner, 0.0);
        let initial_b = Vec2::new(outer, 0.0);
        let mat = Mat2::from_angle(angle_range.start);
        let a = center + mat * initial_a;
        let b = center + mat * initial_b;

        let vertices = &mut self.mesh.vertices;

        // Use extend instead of loop to allow pre-allocation.
        // Calculate index before extending vertices.
        let starting_index = vertices.len();
        self.mesh
            .indices
            .extend((0..segments).into_iter().flat_map(|i| {
                let index = starting_index + i * 2;
                // Triangles are [A, D, B] and [A, C, D].
                array::IntoIter::new([
                    I::from_usize(index),
                    I::from_usize(index + 3),
                    I::from_usize(index + 1),
                    I::from_usize(index),
                    I::from_usize(index + 2),
                    I::from_usize(index + 3),
                ])
            }));

        // Use extend instead of loop to allow pre-allocation.
        let angle_per_segment = angle_span / segments as f32;
        vertices.extend(
            array::IntoIter::new([PosColor { pos: a, color }, PosColor { pos: b, color }]).chain(
                (1..=segments).into_iter().flat_map(|i| {
                    let angle = i as f32 * angle_per_segment + angle_range.start;
                    let mat = Mat2::from_angle(angle);

                    let c = center + mat * initial_a;
                    let d = center + mat * initial_b;

                    array::IntoIter::new([PosColor { pos: c, color }, PosColor { pos: d, color }])
                }),
            ),
        );
    }

    /// add_circle_graphic adds a circle outline to the graphics queue.
    pub fn add_circle(&mut self, center: Vec2, radius: f32, thickness: f32, color: Vec4) {
        self.add_arc(
            center,
            radius,
            0.0..std::f32::consts::PI * 2.0,
            thickness,
            color,
        );
    }

    /// Like `Self::add_circle_graphic` but filled instead of hollow.
    pub fn add_filled_circle(&mut self, center: Vec2, radius: f32, color: Vec4) {
        // TODO: Not the most efficient way to make a filled circle.
        self.add_circle(center, radius * 0.5, radius, color)
    }
}

impl<I: Index> Layer for GraphicLayer<I> {
    fn pre_prepare(&mut self, renderer: &Renderer) {
        self.zoom = renderer.camera.zoom();
    }

    fn render(&mut self, renderer: &Renderer) {
        if self.mesh.is_empty() {
            return;
        }

        if let Some(shader) = renderer.bind_shader(renderer.graphic_shader.as_ref().unwrap()) {
            shader.uniform_matrix3f("uView", &renderer.camera.view_matrix);

            self.buffer.buffer_mesh(&renderer.gl, &self.mesh);
            self.buffer
                .bind(&renderer.gl, &renderer.oes_vao)
                .draw(Gl::TRIANGLES);

            self.mesh.clear();
        }
    }
}
