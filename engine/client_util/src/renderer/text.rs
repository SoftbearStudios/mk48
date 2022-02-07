// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::buffer::RenderBuffer;
use crate::renderer::renderer::Layer;
use crate::renderer::renderer::Renderer;
use crate::renderer::shader::Shader;
use crate::renderer::texture::Texture;
use glam::{vec2, Mat3, Vec2, Vec4};
use std::collections::HashMap;
use web_sys::WebGlRenderingContext as Gl;

/// Renders text on screen.
pub struct TextLayer {
    /// Buffer draw calls.
    draws: Vec<(TextId, Vec2, f32)>,
    /// Too expensive to create text textures every frame, so cache them.
    cache: HashMap<TextId, (Texture, u8)>,
    /// Same for all text.
    text_geometry: RenderBuffer<Vec2>,
}

/// 8 bit rbga color (compatible with JS).
type Color = [u8; 4];

/// Each TextId maps to a unique texture.
type TextId = (String, Color);

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
                vec2(-0.5, 0.5),
                vec2(0.5, 0.5),
                vec2(-0.5, -0.5),
                vec2(0.5, -0.5),
            ],
            &[2, 0, 1, 2, 1, 3],
        );

        Self {
            draws: Vec::new(),
            cache: HashMap::new(),
            text_geometry,
        }
    }

    /// Adds text to the rendering queue.
    pub fn add(&mut self, text: String, position: Vec2, scale: f32, color: Vec4) {
        if text.is_empty() {
            return;
        }

        // TODO take integer based color as input.
        let color = color.to_array().map(|c| (c * 255.0) as u8);
        if color[3] == 0 {
            return;
        }

        // Compensate for resizing text texture to 36 pixels to fit "😊".
        // TODO find better solution.
        let scale = scale * (36.0 / 32.0);

        self.draws.push(((text, color), position, scale));
    }
}

impl Layer for TextLayer {
    fn pre_render(&mut self, renderer: &Renderer) {
        // Sort draws by texture for faster rendering.
        // Needs to be stable sort for deterministic blending of similar text objects.
        self.draws.sort_by(|a, b| a.0.cmp(&b.0));

        // Generate textures here to avoid pipeline stall if done during rendering.
        for (id, ..) in self.draws.iter() {
            let (_, (_, counter)) = self.cache.raw_entry_mut().from_key(id).or_insert_with(|| {
                (
                    id.clone(),
                    (Texture::from_str_and_color(&renderer.gl, &id.0, id.1), 0),
                )
            });
            *counter = 0;
        }

        // Removes textures that haven't been used in 255 frames.
        self.cache.drain_filter(|_, (_texture, counter)| {
            if let Some(next) = counter.checked_add(1) {
                *counter = next;
                false
            } else {
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
            let mut last_id = TextId::default();
            let mut texture_aspect = 0.0;

            for (id, position, scale) in self.draws.drain(..) {
                if id != last_id {
                    let texture = &self.cache[&id].0;
                    shader.uniform_texture("uSampler", texture, 0);

                    texture_aspect = texture.aspect();
                    last_id = id;
                }

                let mat = Mat3::from_scale_angle_translation(
                    vec2(scale * texture_aspect, scale),
                    0.0,
                    position,
                );
                shader.uniform_matrix3f("uView", &(renderer.camera.view_matrix * mat));

                buffer.draw(Gl::TRIANGLES);
            }
        }
    }
}
