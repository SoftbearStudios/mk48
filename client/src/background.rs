// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::sprite::SortableSprite;
use client_util::renderer::background::BackgroundContext;
use client_util::renderer::renderer::Renderer;
use client_util::renderer::shader::ShaderBinding;
use client_util::renderer::texture::Texture;
use common::entity::{EntityId, EntityType};
use common::terrain;
use common::terrain::{Coord, Terrain};
use common::transform::Transform;
use common::velocity::Velocity;
use common_util::angle::{Angle, AngleRepr};
use glam::{vec2, Mat3, UVec2, Vec2, Vec3};

pub struct Mk48OverlayContext {
    visual_range: f32,
    visual_restriction: f32,
    world_radius: f32,
}

impl Default for Mk48OverlayContext {
    fn default() -> Self {
        Self {
            visual_range: 0.0,
            visual_restriction: 0.0,
            world_radius: 1000.0,
        }
    }
}

pub struct Mk48BackgroundContext {
    terrain_texture: Option<Texture>,
    sand_texture: Texture,
    grass_texture: Texture,
    matrix: Mat3,
    time: f32,
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
        }
    }

    pub fn update(
        &mut self,
        camera: Vec2,
        zoom: f32,
        time_seconds: f32,
        ter: &Terrain,
        renderer: &Renderer,
    ) -> impl Iterator<Item = SortableSprite> {
        // Both width and height must be odd numbers so there is an equal distance from the center
        // on both sides.
        let terrain_width: usize = 2 * ((zoom / terrain::SCALE).max(2.0) as usize + 1) + 3;
        let terrain_height =
            2 * ((zoom / (renderer.aspect_ratio() * terrain::SCALE)).max(2.0) as usize + 1) + 3;

        let terrain_center = Coord::from_position(camera).unwrap();

        let terrain_bytes: Vec<_> = ter
            .iter_rect_or(terrain_center, terrain_width, terrain_height, 0)
            .collect();

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
        generate_vegetation(terrain_bytes, terrain_center, terrain_width, terrain_height)
    }
}

impl BackgroundContext for Mk48BackgroundContext {
    fn prepare(&mut self, shader: &mut ShaderBinding) {
        shader.uniform_texture("uSampler", self.terrain_texture.as_ref().unwrap(), 0);
        // TODO don't bind textures if not enabled.
        shader.uniform_texture("uSand", &self.sand_texture, 1);
        shader.uniform_texture("uGrass", &self.grass_texture, 2);
        shader.uniform_matrix3f("uTexture", &self.matrix);
        shader.uniform1f("uTime", self.time);
    }
}

impl Mk48OverlayContext {
    pub fn update(&mut self, visual_range: f32, visual_restriction: f32, world_radius: f32) {
        self.visual_range = visual_range;
        self.visual_restriction = visual_restriction;
        self.world_radius = world_radius;
    }
}

impl BackgroundContext for Mk48OverlayContext {
    fn prepare(&mut self, shader: &mut ShaderBinding) {
        shader.uniform3f(
            "uVisual_uRestrict_uBorder",
            Vec3::new(
                self.visual_range,
                self.visual_restriction,
                self.world_radius,
            ),
        );
    }
}

/// Generates trees, coral, etc. for visible terrain.
fn generate_vegetation(
    terrain_bytes: Vec<u8>,
    center: Coord,
    width: usize,
    height: usize,
) -> impl Iterator<Item = SortableSprite> {
    // Must be power of 2.
    let step = 2usize.pow(2);
    let step_log2 = (step - 1).count_ones();

    // Round starting coords down to step.
    let start_x = center.0 as isize - (width / 2) as isize & !(step as isize - 1);
    let start_y = center.1 as isize - (height / 2) as isize & !(step as isize - 1);

    // Don't need to round down because step by already handles it.
    let end_x = center.0 as isize + ((width + 1) / 2) as isize;
    let end_y = center.1 as isize + ((height + 1) / 2) as isize;

    (start_y..end_y)
        .step_by(step)
        .flat_map(move |j| (start_x..end_x).step_by(step).map(move |i| (i, j)))
        .filter_map(move |(mut x, mut y)| {
            // Integer based random that will reproduce on all clients.
            let hash = hash_coord(x, y);

            // Nudge coord by a little (still pretty local and won't ever have duplicates).
            let dx = hash % step as u32;
            let dy = (hash >> step_log2) % step as u32;

            x += dx as isize;
            y += dy as isize;

            let i = (x - center.0 as isize) + (width / 2) as isize;
            let j = (y - center.1 as isize) + (height / 2) as isize;

            // Out of bounds because of nudge.
            if i < 0 || i >= width as isize || j < 0 || j >= height as isize {
                return None;
            }

            let i = i as usize;
            let j = j as usize;

            let index = i + j * width;
            let v = terrain_bytes[index];

            // Trees only exist on land.
            if v <= 10 * 16 {
                return None;
            }

            let mut position = terrain::signed_coord_corner(x, y);

            // Use remainder of hash to randomize position and direction.
            // Half of least significant bit already used by nudge.
            let [dx, dy, da, _] = hash.to_le_bytes();

            // Generate within the center of the terrain pixel.
            position += (vec2(dx as f32, dy as f32) - 127.5) * (terrain::SCALE * 0.5 / 255.0);
            let direction = Angle((da as AngleRepr) << 8);

            let transform = Transform {
                position,
                direction,
                velocity: Velocity::ZERO,
            };

            // TODO make don't use a fake entity.
            Some(SortableSprite::new_entity(
                EntityId::new(u32::MAX).unwrap(),
                EntityType::Acacia,
                transform,
                0.0,
                1.0,
            ))
        })
}

// Hashes a coordinate to a u32.
// Repeats every 2^16.
fn hash_coord(x: isize, y: isize) -> u32 {
    hash(x as u16 as u32 + ((y as u16 as u32) << 16))
}

// Hashes a u32 to another u32.
// Based on wyhash: https://docs.rs/wyhash/latest/wyhash/
fn hash(mut s: u32) -> u32 {
    s = s.wrapping_mul(0x78bd_642f);
    let s2 = s ^ 0xa0b4_28db;
    let r = u64::from(s) * u64::from(s2);
    ((r >> 32) ^ r) as u32
}
