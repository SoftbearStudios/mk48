// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Params;
use crate::settings::ShadowSetting;
use glam::{Mat3, Vec2, Vec4};
use renderer::{
    derive_vertex, Layer, MeshBuilder, RenderLayer, Renderer, Shader, Texture, TextureFormat,
    TriangleBuffer,
};
use renderer3d::ShadowResult;
use sprite_sheet::UvSpriteSheet;

derive_vertex!(
    struct SpriteVertex {
        pos: Vec4, // X, Y, altitude, height
        uv: Vec2,
        alpha: f32,
        tangent: Vec2,
    }
);

/// Draws sprites from a [`UvSpriteSheet`].
pub struct SpriteLayer {
    atlas_color: Texture,
    atlas_normal: Texture,
    buffer: TriangleBuffer<SpriteVertex>,
    mesh: MeshBuilder<SpriteVertex>,
    shader: Shader,
    sheet: UvSpriteSheet,
}

impl SpriteLayer {
    pub fn new(renderer: &Renderer, shadows: ShadowSetting) -> Self {
        let sheet = serde_json::from_str(include_str!("./sprites_webgl.json")).unwrap();

        let atlas_color = Texture::load(
            renderer,
            "/sprites_webgl.png",
            TextureFormat::COLOR_RGBA,
            None,
            false,
        );
        let atlas_normal = Texture::load(
            renderer,
            "/sprites_normal_webgl.png",
            TextureFormat::Rgba { premultiply: false },
            Some([127, 127, 255]), // +Z
            false,
        );

        let mut frag = "#version 300 es\n".to_owned();
        frag += shadows.shader_define();
        frag += include_str!("shaders/sprite.frag");

        let shader = Shader::new(renderer, include_str!("shaders/sprite.vert"), &frag);

        Self {
            atlas_color,
            atlas_normal,
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
        self.sheet
            .animations
            .get(name)
            .unwrap_or_else(|| panic!("{name} does not exist in animations"))
            .len()
    }

    /// Draws a sprite. `angle` is in radians.
    pub fn draw(
        &mut self,
        sprite: &str,
        frame: Option<usize>,
        center: Vec2,
        dimensions: Vec2,
        angle: f32,
        alpha: f32,
        altitude: f32,
        height: f32,
    ) {
        if alpha == 0.0 {
            return; // Reserved for shadows.
        }
        self.draw_inner(
            sprite, frame, center, dimensions, angle, alpha, altitude, height,
        );
    }

    /// Draws a sprite shadow. `angle` is in radians.
    pub fn draw_shadow(
        &mut self,
        sprite: &str,
        frame: Option<usize>,
        center: Vec2,
        dimensions: Vec2,
        angle: f32,
    ) {
        self.draw_inner(sprite, frame, center, dimensions, angle, 0.0, 0.0, 0.0);
    }

    /// Draws a sprite or a shadow depending on if the alpha > 0.0.
    fn draw_inner(
        &mut self,
        sprite: &str,
        frame: Option<usize>,
        center: Vec2,
        dimensions: Vec2,
        angle: f32,
        alpha: f32,
        altitude: f32,
        height: f32,
    ) {
        let sprite = if let Some(frame) = frame {
            let animation = &self.sheet.animations.get(sprite).unwrap();
            &animation[frame]
        } else {
            self.sheet.sprites.get(sprite).expect(sprite)
        };

        // TODO make sprites and entities have same aspect ratio.
        let matrix = Mat3::from_scale_angle_translation(
            Vec2::new(dimensions.x, dimensions.x / sprite.aspect),
            angle,
            center,
        );

        let positions = [
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, -0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(-0.5, 0.5),
        ];

        let normal_matrix = Mat3::from_angle(angle);
        let tangent = normal_matrix.transform_vector2(Vec2::new(1.0, 0.0));
        debug_assert!(tangent.is_normalized());

        self.mesh.vertices.extend(
            IntoIterator::into_iter(positions)
                .zip(sprite.uvs.iter())
                .map(|(pos, &uv)| SpriteVertex {
                    pos: matrix.transform_point2(pos).extend(altitude).extend(height),
                    uv,
                    alpha,
                    tangent,
                }),
        );
    }
}

impl Layer for SpriteLayer {
    const ALPHA: bool = true;
}

impl RenderLayer<&ShadowResult<&Mk48Params>> for SpriteLayer {
    fn render(&mut self, renderer: &Renderer, result: &ShadowResult<&Mk48Params>) {
        if self.mesh.is_empty() {
            return;
        }

        if let Some(shader) = self.shader.bind(renderer) {
            result.prepare_shadows(&shader);
            let params = &result.params;

            params.camera.prepare(&shader);
            shader.uniform("uColor", &self.atlas_color);
            shader.uniform("uNormal", &self.atlas_normal);
            shader.uniform("uSun", params.weather.sun);

            self.mesh.push_default_quads();
            self.buffer.buffer_mesh(renderer, &self.mesh);

            self.buffer.bind(renderer).draw();
        }

        // Always clear mesh even if shader wasn't bound.
        self.mesh.clear();
    }
}
