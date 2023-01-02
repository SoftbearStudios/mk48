// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::camera_2d::Camera2d;
use glam::{vec2, Mat3, Vec2};
use renderer::{DefaultRender, Layer, RenderLayer, Renderer, Shader, Texture, TriangleBuffer};
use std::collections::HashMap;
use std::hash::BuildHasher;

#[derive(Default)]
struct Buffers {
    counter: u8,
    draws: Vec<Draw>,
    texture: Option<Texture>,
}

struct Draw {
    center: Vec2,
    scale: f32,
}

/// Draws single lines of text.
pub struct TextLayer {
    /// Too expensive to create text textures every frame, so cache them.
    /// Index on text and color to allow CanvasRenderingContext to apply correct coloring to emojis.
    /// Uses 8 bit rbga color (compatible with JS).
    /// TODO could use additive blend mode to prevent unstable ordering if it matters.
    buffers: HashMap<(String, [u8; 4]), Buffers>,
    /// Same for all text.
    geometry: TriangleBuffer<Vec2>,
    shader: Shader,
}

impl DefaultRender for TextLayer {
    fn new(renderer: &Renderer) -> Self {
        let shader = renderer.create_shader(
            include_str!("shaders/text.vert"),
            include_str!("shaders/text.frag"),
        );

        let mut text_geometry = TriangleBuffer::new(renderer);
        text_geometry.buffer(
            renderer,
            &[
                vec2(-0.5, -0.5),
                vec2(0.5, -0.5),
                vec2(0.5, 0.5),
                vec2(-0.5, 0.5),
            ],
            &[0, 1, 2, 2, 3, 0],
        );

        Self {
            buffers: HashMap::new(),
            geometry: text_geometry,
            shader,
        }
    }
}

impl TextLayer {
    /// Draws `text` centered at `center` with a `scale` and a `color`. TODO `scale`'s units need
    /// to be more precisely defined.
    pub fn draw(&mut self, text: &str, center: Vec2, scale: f32, color: [u8; 4]) {
        if text.is_empty() {
            return;
        }

        if color[3] == 0 {
            return;
        }

        // Compensate for resizing text texture to 36 pixels to fit "😊".
        // TODO find better solution.
        let scale = scale * (36.0 / 32.0);

        // Save String allocation most of the time.
        // Can't use .from_key because can't implement the [`std::borrow::Borrow`] trait.
        let hash = self.buffers.hasher().hash_one((text, color));
        let (_, entry) = self
            .buffers
            .raw_entry_mut()
            .from_hash(hash, |existing| {
                existing.0.as_str() == text && existing.1 == color
            })
            .or_insert_with(|| ((text.to_owned(), color), Default::default()));

        entry.draws.push(Draw { center, scale });
    }
}

impl Layer for TextLayer {
    const ALPHA: bool = true;

    fn pre_render(&mut self, renderer: &Renderer) {
        self.buffers.retain(|id, entry| {
            entry.texture.get_or_insert_with(|| {
                // Generate textures here to avoid pipeline stall if done during rendering.
                Texture::from_text(renderer, &id.0, id.1)
            });

            // Remove textures that haven't been used in 255 (u8::MAX) frames.
            if entry.draws.is_empty() {
                if let Some(next) = entry.counter.checked_add(1) {
                    entry.counter = next;
                    true // Keep alive (was used recently).
                } else {
                    false // Destroy (wasn't used in a few seconds).
                }
            } else {
                entry.counter = 0;
                true // Keep alive (was used this frame).
            }
        });
    }
}

impl RenderLayer<&Camera2d> for TextLayer {
    fn render(&mut self, renderer: &Renderer, camera: &Camera2d) {
        // Haven't rendered text in a while.
        if self.buffers.is_empty() {
            return;
        }

        if let Some(shader) = self.shader.bind(renderer) {
            let binding = self.geometry.bind(renderer);

            for buffers in self.buffers.values_mut() {
                if buffers.draws.is_empty() {
                    continue; // Nothing to draw.
                }

                // Shouldn't panic because texture was initialized in pre_render.
                let texture = buffers.texture.as_ref().unwrap();
                let texture_aspect = texture.aspect();
                shader.uniform("uSampler", texture);

                // TODO could draw multiple in a single draw call.
                for Draw { center, scale } in buffers.draws.drain(..) {
                    let model = Mat3::from_scale_angle_translation(
                        vec2(scale * texture_aspect, scale),
                        0.0,
                        center,
                    );
                    // Only drawing 1 at a time so we can premultiply the model and view matrix.
                    shader.uniform("uModelView", &(camera.view_matrix * model));
                    binding.draw();
                }
            }
        }
    }
}
