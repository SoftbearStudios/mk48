// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::buffer::RenderBuffer;
use crate::renderer::renderer::Layer;
use crate::renderer::renderer::Renderer;
use crate::renderer::shader::Shader;
use crate::renderer::texture::Texture;
use glam::{Mat3, Vec2, Vec4};
use std::collections::HashMap;
use std::ops::Mul;
use web_sys::WebGlRenderingContext as Gl;

/// Renders text on screen.
pub struct TextLayer {
    /// Buffer draw calls.
    draws: Vec<(String, Vec2, f32, Vec4)>,
    /// Too expensive to create text textures every frame, so cache them.
    cache: HashMap<String, (Texture, u8)>,
    /// Same for all text.
    text_geometry: RenderBuffer<Vec2>,
}

impl TextLayer {
    pub fn new(renderer: &mut Renderer) -> Self {
        let gl = &renderer.gl;
        renderer.text_shader.get_or_insert_with(|| {
            Shader::new(
                gl,
                include_str!("./shaders/text.vert"),
                include_str!("./shaders/text.frag"),
            )
        });

        let mut text_geometry = RenderBuffer::new(&renderer.gl, &renderer.oes_vao);
        text_geometry.buffer(
            &renderer.gl,
            &[
                Vec2::new(-0.5, 0.5),
                Vec2::new(0.5, 0.5),
                Vec2::new(-0.5, -0.5),
                Vec2::new(0.5, -0.5),
            ],
            &[2, 0, 1, 2, 1, 3],
        );

        Self {
            draws: Vec::new(),
            cache: HashMap::new(),
            text_geometry,
        }
    }

    /// Adds text to the rendering queue. Text will
    pub fn add(&mut self, text: String, position: Vec2, scale: f32, color: Vec4) {
        self.draws.push((text, position, scale, color));
    }
}

impl Layer for TextLayer {
    fn pre_render(&mut self, renderer: &Renderer) {
        for (text, ..) in self.draws.iter() {
            let (_, counter) = self
                .cache
                .entry(text.to_owned())
                .or_insert_with(|| (Texture::from_str(&renderer.gl, text), 0));
            *counter = 0;
        }

        self.cache.drain_filter(|_text, (_texture, counter)| {
            if let Some(next) = counter.checked_add(1) {
                *counter = next;
                false
            } else {
                //crate::console_log!("Free: {}", text);
                true
            }
        });
    }

    fn render(&mut self, renderer: &Renderer) {
        if self.draws.is_empty() {
            return;
        }

        if let Some(shader) = renderer.bind_shader(renderer.text_shader.as_ref().unwrap()) {
            let buffer = renderer.bind_buffer(&self.text_geometry);

            for (name, position, scale, color) in self.draws.drain(..) {
                let texture = &self.cache[&name].0;
                shader.uniform_texture("uSampler", texture, 0);

                let mat = Mat3::from_scale_angle_translation(
                    Vec2::new(scale / texture.aspect, scale),
                    0.0,
                    position,
                );

                shader.uniform_matrix3f("uView", &renderer.view_matrix.mul(mat));
                shader.uniform4f("uColor", color);

                buffer.draw(Gl::TRIANGLES);
            }
        }
    }
}
