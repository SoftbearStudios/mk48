// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::buffer::{MeshBuffer, RenderBuffer};
use crate::renderer::renderer::Layer;
use crate::renderer::renderer::Renderer;
use crate::renderer::shader::Shader;
use crate::renderer::texture::Texture;
use crate::renderer::vertex::PosUvAlpha;
use common_util::angle::Angle;
use glam::{Mat3, Vec2};
use sprite_sheet::UvSpriteSheet;
use std::array;
use web_sys::WebGlRenderingContext as Gl;

/// Renders sprites from a spritesheet.
pub struct SpriteLayer {
    mesh: MeshBuffer<PosUvAlpha>,
    buffer: RenderBuffer<PosUvAlpha>,
    texture: Texture,
    sheet: UvSpriteSheet,
}

impl SpriteLayer {
    pub fn new(renderer: &mut Renderer, texture: Texture, sheet: UvSpriteSheet) -> Self {
        let gl = &renderer.gl;
        renderer.sprite_shader.get_or_insert_with(|| {
            Shader::new(
                gl,
                include_str!("./shaders/sprite.vert"),
                include_str!("./shaders/sprite.frag"),
            )
        });

        Self {
            texture,
            mesh: MeshBuffer::new(),
            buffer: RenderBuffer::new(&renderer.gl, &renderer.oes_vao),
            sheet,
        }
    }

    /// Gets length of named animation in frames.
    ///
    /// # Panics
    ///
    /// If the animation doesn't exist.
    pub fn animation_length(&self, name: &str) -> usize {
        self.sheet.animations.get(name).unwrap().len()
    }

    /// Adds a sprite to the rendering queue. Sprites will be rendered in the order they are added.
    pub fn add(
        &mut self,
        sprite: &str,
        animation_frame: Option<usize>,
        center: Vec2,
        dimensions: Vec2,
        angle: Angle,
        alpha: f32,
    ) {
        let sprite = if let Some(frame) = animation_frame {
            let animation = &self.sheet.animations.get(sprite).unwrap();
            &animation[frame]
        } else {
            &self.sheet.sprites.get(sprite).expect(sprite)
        };

        let matrix = Mat3::from_scale_angle_translation(
            Vec2::new(dimensions.x, dimensions.x * sprite.aspect),
            angle.to_radians(),
            center,
        );

        let positions = [
            Vec2::new(-0.5, 0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, -0.5),
        ];

        self.mesh
            .vertices
            .extend(
                array::IntoIter::new(positions)
                    .zip(sprite.uvs.iter())
                    .map(|(pos, &uv)| PosUvAlpha {
                        pos: matrix.transform_point2(pos),
                        uv,
                        alpha,
                    }),
            );
    }
}

impl Layer for SpriteLayer {
    fn render(&mut self, renderer: &Renderer) {
        if self.mesh.is_empty() {
            return;
        }

        if let Some(shader) = renderer.bind_shader(renderer.sprite_shader.as_ref().unwrap()) {
            shader.uniform_texture("uSampler", &self.texture, 0);
            shader.uniform_matrix3f("uView", &renderer.view_matrix);

            self.mesh.push_default_quads();
            self.buffer.buffer_mesh(&renderer.gl, &self.mesh);

            self.buffer
                .bind(&renderer.gl, &renderer.oes_vao)
                .draw(Gl::TRIANGLES);
        }

        self.mesh.clear();
    }
}
