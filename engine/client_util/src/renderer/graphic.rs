// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::buffer::{Index, MeshBuffer, RenderBuffer};
use crate::renderer::renderer::Layer;
use crate::renderer::renderer::Renderer;
use crate::renderer::shader::Shader;
use crate::renderer::vertex::PosColor;
use glam::{Mat2, Vec2, Vec4};
use std::array;
use std::ops::Range;
use web_sys::WebGlRenderingContext as Gl;

/// Renders solid color polygons, known as graphics.
pub struct GraphicLayer {
    mesh: MeshBuffer<PosColor>,
    buffer: RenderBuffer<PosColor>,
    /// Zoom value from last frame, useful for generating curves with appropriate segments.
    last_zoom: f32,
}

impl GraphicLayer {
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
            last_zoom: 1.0,
        }
    }

    /// add_triangle_graphic adds a transformed equilateral triangle to the graphics queue, pointing
    /// upward if angle is zero.
    pub fn add_triangle(&mut self, center: Vec2, scale: Vec2, angle: f32, color: Vec4) {
        let index = self.mesh.vertices.len();
        self.mesh.indices.extend_from_slice(&[
            index as Index,
            index as Index + 1,
            index as Index + 2,
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
        let index = self.mesh.vertices.len();
        self.mesh.push_quad([
            index as Index,
            index as Index + 1,
            index as Index + 2,
            index as Index + 3,
        ]);

        let half_scale = scale * 0.5;
        let rot = Mat2::from_angle(angle);
        let positions = [
            Vec2::new(-half_scale.x, half_scale.y),
            Vec2::new(half_scale.x, half_scale.y),
            Vec2::new(-half_scale.x, -half_scale.y),
            Vec2::new(half_scale.x, -half_scale.y),
        ];

        self.mesh.vertices.extend(positions.map(|pos| PosColor {
            pos: center + rot * pos,
            color,
        }));
    }

    /// add_rectangle_graphic adds a line to the graphics queue.
    pub fn add_line(&mut self, start: Vec2, end: Vec2, thickness: f32, color: Vec4) {
        let diff = end - start;
        let angle = diff.y.atan2(diff.x);

        self.add_rectangle(
            start + diff * 0.5,
            Vec2::new(diff.length(), thickness),
            angle,
            color,
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
        assert!(radius > 0.0, "radius must be positive");

        let angle_span = angle_range.end - angle_range.start;
        if angle_span <= 0.0 {
            // Nothing to draw.
            return;
        }

        // Number of segments to approximate an arc.
        // The radius.sqrt() helps even out the quality surprisingly well.
        let segments = ((radius / self.last_zoom).sqrt()
            * angle_span
            * (200.0 / (std::f32::consts::PI * 2.0))) as i32;

        // Set maximum to prevent indices from overflowing.
        let segments = segments.clamp(6, 100) as u32;

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
        let starting_index = vertices.len() as u32;
        self.mesh
            .indices
            .extend((0..segments).into_iter().flat_map(|i| {
                let index = (starting_index + i * 2) as Index;
                // Triangles are [A, D, B] and [A, C, D].
                array::IntoIter::new([index, index + 3, index + 1, index, index + 2, index + 3])
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

impl Layer for GraphicLayer {
    fn render(&mut self, renderer: &Renderer) {
        self.last_zoom = renderer.zoom;

        if let Some(shader) = renderer.bind_shader(renderer.graphic_shader.as_ref().unwrap()) {
            shader.uniform_matrix3f("uView", &renderer.view_matrix);

            self.buffer.buffer_mesh(&renderer.gl, &self.mesh);
            self.buffer
                .bind(&renderer.gl, &renderer.oes_vao)
                .draw(Gl::TRIANGLES);

            self.mesh.clear();
        }
    }
}
