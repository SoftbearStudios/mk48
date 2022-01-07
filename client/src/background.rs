// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::state::Mk48State;
use client_util::renderer::background::BackgroundContext;
use client_util::renderer::renderer::Renderer;
use client_util::renderer::shader::ShaderBinding;
use client_util::renderer::texture::Texture;
use common::terrain;
use common::terrain::Coord;
use glam::{Mat3, UVec2, Vec2, Vec4};

pub struct Mk48BackgroundContext {
    terrain_texture: Option<Texture>,
    sand_texture: Texture,
    grass_texture: Texture,
    matrix: Mat3,
    time: f32,
    visual_range: f32,
    visual_restriction: f32,
    world_radius: f32,
}

impl Mk48BackgroundContext {
    pub const SAND_COLOR: [u8; 3] = [213, 176, 107];
    pub const GRASS_COLOR: [u8; 3] = [71, 85, 45];

    pub fn new(render_terrain_textures: bool, renderer: &Renderer) -> Self {
        let (sand_path, grass_path) = if render_terrain_textures {
            ("/sand.png", "/grass.png")
        } else {
            // TODO: Kludge, don't load textures.
            ("/dummy.png", "/dummy.png")
        };

        Mk48BackgroundContext {
            terrain_texture: None,
            sand_texture: renderer.load_texture(
                sand_path,
                UVec2::new(256, 256),
                Some(Mk48BackgroundContext::SAND_COLOR),
                true,
            ),
            grass_texture: renderer.load_texture(
                grass_path,
                UVec2::new(256, 256),
                Some(Mk48BackgroundContext::GRASS_COLOR),
                true,
            ),
            matrix: Mat3::IDENTITY,
            time: 0.0,
            visual_range: 0.0,
            visual_restriction: 0.0,
            world_radius: 0.0,
        }
    }

    pub fn update(
        &mut self,
        camera: Vec2,
        zoom: f32,
        time_seconds: f32,
        visual_range: f32,
        visual_restriction: f32,
        game_state: &Mk48State,
        renderer: &Renderer,
    ) {
        // Both width and height must be odd numbers so there is an equal distance from the center
        // on both sides.
        let terrain_width: usize = 2 * ((zoom / terrain::SCALE).max(2.0) as usize + 1) + 3;
        let terrain_height =
            2 * ((zoom / (renderer.aspect_ratio() * terrain::SCALE)).max(2.0) as usize + 1) + 3;

        let mut terrain_bytes = Vec::with_capacity(terrain_width * terrain_height);
        let terrain_center = Coord::from_position(camera).unwrap();

        terrain_bytes.extend(game_state.terrain.iter_rect_or(
            terrain_center,
            terrain_width,
            terrain_height,
            0,
        ));

        let terrain_offset = Mat3::from_translation(-terrain_center.corner());
        let terrain_scale = &Mat3::from_scale(Vec2::new(
            1.0 / (terrain_width as f32 * terrain::SCALE),
            1.0 / (terrain_height as f32 * terrain::SCALE),
        ));

        // This matrix converts from world space to terrain texture UV coordinates.
        self.matrix = Mat3::from_translation(Vec2::new(0.5, 0.5))
            .mul_mat3(&terrain_scale.mul_mat3(&terrain_offset));

        renderer.realloc_texture_from_bytes(
            &mut self.terrain_texture,
            terrain_width as u32,
            terrain_height as u32,
            &terrain_bytes,
        );

        self.time = time_seconds;
        self.visual_range = visual_range;
        self.visual_restriction = visual_restriction;
        self.world_radius = game_state.world_radius;
    }
}

impl BackgroundContext for Mk48BackgroundContext {
    fn prepare(&mut self, shader: &mut ShaderBinding) {
        shader.uniform_texture("uSampler", self.terrain_texture.as_ref().unwrap(), 0);
        // TODO don't bind textures if not enabled.
        shader.uniform_texture("uSand", &self.sand_texture, 1);
        shader.uniform_texture("uGrass", &self.grass_texture, 2);
        shader.uniform_matrix3f("uTexture", &self.matrix);
        shader.uniform4f(
            "uTime_uVisual_uRestrict_uBorder",
            Vec4::new(
                self.time,
                self.visual_range,
                self.visual_restriction,
                self.world_radius,
            ),
        );
    }
}
