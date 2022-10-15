// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::camera_2d::Camera2d;
use crate::Renderer2d;
use glam::{Mat3, Vec2};
use renderer::{derive_vertex, Camera, Layer, MeshBuilder, Shader, Texture, TriangleBuffer};
use sprite_sheet::UvSpriteSheet;

derive_vertex!(
    struct PosUvAlpha {
        pos: Vec2,
        uv: Vec2,
        alpha: f32,
    }
);

/// Draws sprites from a [`UvSpriteSheet`].
pub struct SpriteLayer {
    atlas: Texture,
    buffer: TriangleBuffer<PosUvAlpha>,
    mesh: MeshBuilder<PosUvAlpha>,
    shader: Shader,
    sheet: UvSpriteSheet,
}

impl SpriteLayer {
    /// Creates a [`SpriteLayer`] from a [`Texture`] `atlas` and a `sheet` describing where the sprites are on
    /// `atlas`.
    pub fn new(renderer: &Renderer2d, atlas: Texture, sheet: UvSpriteSheet) -> Self {
        let shader = renderer.create_shader(
            include_str!("shaders/sprite.vert"),
            include_str!("shaders/sprite.frag"),
        );

        Self {
            atlas,
            buffer: TriangleBuffer::new(renderer),
            mesh: MeshBuilder::new(),
            shader,
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

    /// Draws a sprite. `angle` is in radians.
    pub fn draw(
        &mut self,
        sprite: &str,
        animation_frame: Option<usize>,
        center: Vec2,
        dimensions: Vec2,
        angle: f32,
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
            angle,
            center,
        );

        let positions = [
            Vec2::new(-0.5, 0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, -0.5),
        ];

        self.mesh.vertices.extend(
            IntoIterator::into_iter(positions)
                .zip(sprite.uvs.iter())
                .map(|(pos, &uv)| PosUvAlpha {
                    pos: matrix.transform_point2(pos),
                    uv,
                    alpha,
                }),
        );
    }
}

impl Layer<Camera2d> for SpriteLayer {
    fn render(&mut self, renderer: &Renderer2d) {
        if self.mesh.is_empty() {
            return;
        }

        if let Some(shader) = self.shader.bind(renderer) {
            renderer.camera.uniform_matrix(&shader);
            shader.uniform_texture("uSampler", &self.atlas, 0);

            self.mesh.push_default_quads();
            self.buffer.buffer_mesh(renderer, &self.mesh);

            self.buffer.bind(&renderer).draw();
        }

        // Always clear mesh even if shader wasn't bound.
        self.mesh.clear();
    }
}
