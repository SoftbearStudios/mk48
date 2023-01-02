// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::camera_2d::Camera2d;
use glam::{Mat2, Vec2, Vec4};
use renderer::{
    derive_vertex, DefaultRender, Index, Layer, MeshBuilder, RenderLayer, Renderer, Shader,
    TriangleBuffer,
};
use std::cmp::Ordering;
use std::f32::consts::PI;
use std::ops::Range;

derive_vertex!(
    struct PosColor {
        pos: Vec2,
        color: Vec4,
    }
);

/// Draws solid color polygons, also known as graphics. All its methods take angles in radians.
pub struct GraphicLayer<I: Index = u16> {
    shader: Shader,
    mesh: MeshBuilder<PosColor, I>,
    buffer: TriangleBuffer<PosColor, I>,
    zoom: f32,
}

impl<I: Index> DefaultRender for GraphicLayer<I> {
    fn new(renderer: &Renderer) -> Self {
        let shader = renderer.create_shader(
            include_str!("shaders/graphic.vert"),
            include_str!("shaders/graphic.frag"),
        );

        Self {
            shader,
            mesh: MeshBuilder::new(),
            buffer: TriangleBuffer::new(renderer),
            zoom: 1.0,
        }
    }
}

impl<I: Index> GraphicLayer<I> {
    /// Draws a triangle centered on `center`, with a base of `scale.x`, a height of `scale.y` and
    /// rotated by `angle`. An `angle` of 0 is pointing
    pub fn draw_triangle(&mut self, center: Vec2, scale: Vec2, angle: f32, color: Vec4) {
        let index = self.mesh.vertices.len();
        self.mesh.indices.extend_from_slice(&[
            I::from_usize(index),
            I::from_usize(index + 1),
            I::from_usize(index + 2),
        ]);

        let rot = Mat2::from_angle(angle);
        let positions = [
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, -0.5),
            Vec2::new(0.0, 0.25 * 3f32.sqrt()),
        ];

        self.mesh.vertices.extend(positions.map(|pos| PosColor {
            pos: center + rot * (pos * scale),
            color,
        }));
    }

    /// Draws a rectangle centered on `center`, with length/width of `scale` and rotated by `angle`.
    /// At `angle` `scale.x` is in the x-axis.
    pub fn draw_rectangle(&mut self, center: Vec2, scale: Vec2, angle: f32, color: Vec4) {
        self.draw_rectangle_gradient(center, scale, angle, [color; 4])
    }

    /// Like [`draw_rectangle`][`Self::draw_rectangle`] but with different colors on each corner.
    pub fn draw_rectangle_gradient(
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
            Vec2::new(-half_scale.x, -half_scale.y),
            Vec2::new(half_scale.x, -half_scale.y),
            Vec2::new(half_scale.x, half_scale.y),
            Vec2::new(-half_scale.x, half_scale.y),
        ];

        self.mesh
            .vertices
            .extend(positions.zip(colors).map(|(pos, color)| PosColor {
                pos: center + rot * pos,
                color,
            }));
    }

    /// Draws a line starting at `start` and ending at `end`.
    pub fn draw_line(&mut self, start: Vec2, end: Vec2, thickness: f32, color: Vec4) {
        self.draw_line_gradient(start, end, thickness, color, color)
    }

    /// Like [`draw_line`][`Self::draw_line`] but with different colors on each end.
    pub fn draw_line_gradient(&mut self, start: Vec2, end: Vec2, thickness: f32, s: Vec4, e: Vec4) {
        let diff = end - start;
        let angle = diff.y.atan2(diff.x);

        self.draw_rectangle_gradient(
            start + diff * 0.5,
            Vec2::new(diff.length(), thickness),
            angle,
            [s, e, e, s],
        );
    }

    /// Like [`draw_line`][`Self::draw_line`] but endpoints are rounded. If `extend`, start and end
    /// will be the centers of the semicircle endpoints (aka longer than a normal line).
    pub fn draw_rounded_line(
        &mut self,
        start: Vec2,
        end: Vec2,
        thickness: f32,
        color: Vec4,
        extend: bool,
    ) {
        self.draw_rounded_line_gradient(start, end, thickness, color, color, extend);
    }

    /// Like [`draw_rounded_line`][`Self::draw_rounded_line`] but with different colors on each end.
    pub fn draw_rounded_line_gradient(
        &mut self,
        start: Vec2,
        end: Vec2,
        mut thickness: f32,
        s: Vec4,
        e: Vec4,
        extend: bool,
    ) {
        let length = start.distance(end);
        let mut line_length = length;

        if !extend {
            // Can't round a line that is shorter than it is wide.
            thickness = thickness.min(length);
            let radius = thickness * 0.5;

            // 2 arcs on either side of line take up 2 radii of length.
            line_length -= radius * 2.0
        }

        // Don't divide by zero.
        let line_factor = if length < 0.0001 {
            0.0
        } else {
            (length - line_length) / length
        };

        let line_start = start.lerp(end, line_factor * 0.5);
        let line_end = end.lerp(start, line_factor * 0.5);

        // Don't add a line if it has zero length.
        if line_start != line_end {
            self.draw_line_gradient(line_start, line_end, thickness, s, e);
        }

        let diff = line_end - line_start;
        let angle = diff.y.atan2(diff.x);

        let arc_thickness = thickness * 0.5;
        // Account for thickness which is applied to radius.
        let adjusted_radius = arc_thickness * 0.5;

        // Rounded start.
        let opp_angle = angle - PI;
        let angle_range = (opp_angle - PI / 2.0)..(opp_angle + PI / 2.0);
        self.draw_arc(line_start, adjusted_radius, angle_range, arc_thickness, s);

        // Rounded end.
        let angle_range = (angle - PI / 2.0)..(angle + PI / 2.0);
        self.draw_arc(line_end, adjusted_radius, angle_range, arc_thickness, e);
    }

    /// Draw an arc.
    pub fn draw_arc(
        &mut self,
        center: Vec2,
        radius: f32,
        angle_range: Range<f32>,
        thickness: f32,
        color: Vec4,
    ) {
        self.draw_arc_inner(center, radius, angle_range, thickness, color, None);
    }

    fn draw_arc_inner(
        &mut self,
        center: Vec2,
        radius: f32,
        angle_range: Range<f32>,
        thickness: f32,
        color: Vec4,
        segments: Option<usize>,
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

        let segments = segments.unwrap_or_else(|| {
            // Number of segments to approximate an arc.
            // The radius.sqrt() helps even out the quality surprisingly well.
            let segments = ((radius / self.zoom).sqrt() * angle_span * (200.0 / (PI * 2.0))) as i32;

            // Set maximum to prevent indices from overflowing.
            segments.clamp(6, 100) as usize
        });

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
                // Triangles are [A, B, D] and [A, D, C].
                IntoIterator::into_iter([
                    I::from_usize(index),
                    I::from_usize(index + 1),
                    I::from_usize(index + 3),
                    I::from_usize(index),
                    I::from_usize(index + 3),
                    I::from_usize(index + 2),
                ])
            }));

        // Use extend instead of loop to allow pre-allocation.
        let angle_per_segment = angle_span / segments as f32;
        vertices.extend(
            IntoIterator::into_iter([PosColor { pos: a, color }, PosColor { pos: b, color }])
                .chain((1..=segments).into_iter().flat_map(|i| {
                    let angle = i as f32 * angle_per_segment + angle_range.start;
                    let mat = Mat2::from_angle(angle);

                    let c = center + mat * initial_a;
                    let d = center + mat * initial_b;

                    IntoIterator::into_iter([
                        PosColor { pos: c, color },
                        PosColor { pos: d, color },
                    ])
                })),
        );
    }

    /// Draws an outlined circle.
    pub fn draw_circle(&mut self, center: Vec2, radius: f32, thickness: f32, color: Vec4) {
        self.draw_arc(center, radius, 0.0..PI * 2.0, thickness, color);
    }

    /// Like [`draw_circle`][`Self::draw_circle`] but filled instead of outlined.
    pub fn draw_filled_circle(&mut self, center: Vec2, radius: f32, color: Vec4) {
        // TODO: Not the most efficient way to make a filled circle.
        self.draw_circle(center, radius * 0.5, radius, color)
    }

    /// Draws an outlined regular polygon.
    pub fn draw_polygon(
        &mut self,
        center: Vec2,
        radius: f32,
        angle: f32,
        thickness: f32,
        color: Vec4,
        sides: usize,
    ) {
        self.draw_arc_inner(
            center,
            radius,
            angle..(angle + PI * 2.0),
            thickness,
            color,
            Some(sides),
        );
    }

    /// Like [`draw_polygon`][`Self::draw_polygon`] but filled instead of outlined.
    pub fn draw_filled_polygon(
        &mut self,
        center: Vec2,
        radius: f32,
        angle: f32,
        color: Vec4,
        sides: usize,
    ) {
        // TODO: Not the most efficient way to make a filled polygon.
        self.draw_polygon(center, radius * 0.5, angle, radius, color, sides)
    }
}

impl<I: Index> Layer for GraphicLayer<I> {
    const ALPHA: bool = true;
}

impl<I: Index> RenderLayer<&Camera2d> for GraphicLayer<I> {
    fn render(&mut self, renderer: &Renderer, camera: &Camera2d) {
        if self.mesh.is_empty() {
            return;
        }

        if let Some(shader) = self.shader.bind(renderer) {
            camera.prepare(&shader);

            self.buffer.buffer_mesh(renderer, &self.mesh);
            self.buffer.bind(renderer).draw();
        }

        // Always clear mesh even if shader wasn't bound.
        self.mesh.clear();

        // TODO more accurate zoom.
        self.zoom = camera.zoom;
    }
}
