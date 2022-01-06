// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::buffer::RenderBuffer;
use crate::renderer::renderer::{Layer, Renderer};
use crate::renderer::shader::{Shader, ShaderBinding};
use glam::{Vec2, Vec4};
use web_sys::WebGlRenderingContext as Gl;

/// A whole-screen layer.
pub struct BackgroundLayer<C: BackgroundContext = ()> {
    shader: Shader,
    pub context: C,
}

/// Any extra data that is necessary to render the background.
pub trait BackgroundContext {
    fn prepare(&mut self, shader: &mut ShaderBinding);
}

impl BackgroundContext for () {
    fn prepare(&mut self, _shader: &mut ShaderBinding) {}
}

impl<C: BackgroundContext> BackgroundLayer<C> {
    /// Shader must take uCamera and uMiddle_uDerivative uniforms.
    pub fn new(renderer: &mut Renderer, shader: Shader, context: C) -> Self {
        let gl = &renderer.gl;
        let oes_vao = &renderer.oes_vao;
        renderer.background_buffer.get_or_insert_with(|| {
            let mut background_geometry = RenderBuffer::new(gl, oes_vao);
            background_geometry.buffer(
                gl,
                &[
                    Vec2::new(-1.0, 1.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(-1.0, -1.0),
                    Vec2::new(1.0, -1.0),
                ],
                &[2, 0, 1, 2, 1, 3],
            );
            background_geometry
        });

        Self { shader, context }
    }
}

impl<C: BackgroundContext> Layer for BackgroundLayer<C> {
    /// The background shader will be applied to the entire screen.
    /// Prepare can bind any other necessary uniforms.
    fn render(&mut self, renderer: &Renderer) {
        if let Some(mut shader) = self.shader.bind(&renderer.gl, renderer.khr.as_ref()) {
            shader.uniform_matrix3f("uCamera", &renderer.camera_matrix);
            let viewport_meters = (renderer.camera_matrix.transform_point2(Vec2::new(1.0, 1.0))
                - renderer
                    .camera_matrix
                    .transform_point2(Vec2::new(-1.0, -1.0)))
            .abs();
            let derivative = viewport_meters / renderer.canvas_size().as_vec2();

            // Pack multiple uniforms together.
            shader.uniform4f(
                "uMiddle_uDerivative",
                Vec4::new(
                    renderer.center.x,
                    renderer.center.y,
                    derivative.x,
                    derivative.y,
                ),
            );

            self.context.prepare(&mut shader);

            let buffer = renderer.background_buffer.as_ref().unwrap();

            buffer
                .bind(&renderer.gl, &renderer.oes_vao)
                .draw(Gl::TRIANGLES);
        }
    }
}
