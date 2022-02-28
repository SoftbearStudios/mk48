// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::sprite::SortableSprite;
use client_util::renderer::background::{BackgroundContext, Invalidation};
use client_util::renderer::renderer::Renderer;
use client_util::renderer::shader::{Shader, ShaderBinding};
use client_util::renderer::texture::{Texture, TextureFormat};
use common::entity::{EntityId, EntityType};
use common::terrain::{Coord, RelativeCoord, Terrain};
use common::transform::Transform;
use common::velocity::Velocity;
use common::{terrain, world};
use common_util::angle::{Angle, AngleRepr};
use glam::{uvec2, vec2, vec3, Mat3, UVec2, Vec2};
use std::convert::TryInto;

#[derive(Copy, Clone, Default, Eq, PartialEq)]
struct TerrainView {
    center: Coord,
    dimensions: UVec2,
}

impl TerrainView {
    fn new(camera: Vec2, aspect: f32, zoom: f32) -> Self {
        let center = Coord::from_position(camera).unwrap();

        // Both width and height must be odd numbers so there is an equal distance from the center
        // on both sides.
        let width = (2 * ((zoom / terrain::SCALE).max(2.0) as usize + 1) + 3)
            .try_into()
            .unwrap();
        let height = (2 * ((zoom / (aspect * terrain::SCALE)).max(2.0) as usize + 1) + 3)
            .try_into()
            .unwrap();

        Self {
            center,
            dimensions: uvec2(width, height),
        }
    }

    /// Returns a matrix that translates world space to terrain texture UV coordinates.
    fn world_space_to_uv_space(&self) -> Mat3 {
        let offset = Mat3::from_translation(-self.center.corner());
        let scale = &Mat3::from_scale((self.dimensions.as_vec2() * terrain::SCALE).recip());
        Mat3::from_translation(Vec2::splat(0.5)).mul_mat3(&scale.mul_mat3(&offset))
    }
}

pub struct Mk48BackgroundContext {
    terrain_texture: Texture,
    grass_texture: Texture,
    sand_texture: Texture,
    snow_texture: Texture,
    wave_quality: u8,
    animations: bool,
    last_view: TerrainView,
    last_terrain: Vec<u8>,
    last_vegetation: Vec<SortableSprite>,
    invalidation: Option<Invalidation>,
}

impl Mk48BackgroundContext {
    const GRASS_COLOR: [u8; 3] = [71, 85, 45];
    const SAND_COLOR: [u8; 3] = [213, 176, 107];
    const SNOW_COLOR: [u8; 3] = [233, 235, 237];

    pub fn new(renderer: &Renderer, animations: bool, wave_quality: u8) -> Self {
        let terrain_texture = renderer.new_empty_texture(TextureFormat::Alpha, true);

        let grass_texture = renderer.load_texture(
            "/grass.png",
            UVec2::splat(512),
            Some(Mk48BackgroundContext::GRASS_COLOR),
            true,
        );
        let sand_texture = renderer.load_texture(
            "/sand.png",
            UVec2::splat(512),
            Some(Mk48BackgroundContext::SAND_COLOR),
            true,
        );
        let snow_texture = renderer.load_texture(
            "/snow.png",
            UVec2::splat(512),
            Some(Mk48BackgroundContext::SNOW_COLOR),
            true,
        );

        Mk48BackgroundContext {
            terrain_texture,
            grass_texture,
            sand_texture,
            snow_texture,
            wave_quality,
            animations,
            last_view: TerrainView::default(),
            last_terrain: vec![],
            last_vegetation: vec![],
            invalidation: None,
        }
    }

    pub fn update(
        &mut self,
        camera: Vec2,
        zoom: f32,
        terrain: &mut Terrain,
        renderer: &Renderer,
    ) -> impl Iterator<Item = SortableSprite> + '_ {
        let view = TerrainView::new(camera, renderer.aspect_ratio(), zoom);
        let view_changed = view != self.last_view;

        // TODO Only if update happened in our current view.
        let terrain_changed = !terrain.updated.is_empty();

        // If terrain changed or view changed the bytes can change.
        if terrain_changed || view_changed {
            // Reuse previous allocation.
            self.last_terrain.clear();
            self.last_terrain.extend(terrain.iter_rect_or(
                view.center,
                view.dimensions.x as usize,
                view.dimensions.y as usize,
                0,
            ));

            renderer.realloc_texture_with_opt_bytes(
                &mut self.terrain_texture,
                view.dimensions,
                Some(&self.last_terrain),
            );

            // Vegetation only changes if any of its arguments change.
            // Reuse previous allocation.
            self.last_vegetation.clear();
            self.last_vegetation
                .extend(generate_vegetation(&self.last_terrain, view))
        }

        // Only invalidate if terrain changed in the intersection of our current and last views.
        if self.frame_cache_enabled() {
            let updated = terrain.updated.clone();
            if !updated.is_empty() {
                // Only invalidate the rects where pixels could have possibly changed.
                let rects = updated
                    .into_iter()
                    .flat_map(|chunk_id| {
                        let coord = chunk_id.as_coord();
                        terrain
                            .get_chunk(chunk_id)
                            .updated_rects()
                            .map(move |(start, end)| {
                                let end = end + RelativeCoord(1, 1);
                                let offset = -terrain::SCALE * 1.0;

                                let s = (coord + start).corner() + offset;
                                let e = (coord + end).corner() + offset;

                                (s, e)
                            })
                    })
                    .collect();

                self.invalidation = Some(Invalidation::Rects(rects))
            }
        }

        // Finish updates.
        terrain.clear_updated();
        self.last_view = view;

        self.last_vegetation.iter().copied()
    }
}

impl BackgroundContext for Mk48BackgroundContext {
    fn create_shader(&self, renderer: &mut Renderer) -> Shader {
        let background_frag_template = include_str!("./shaders/background.frag");
        let mut background_frag_source = String::with_capacity(background_frag_template.len() + 40);

        background_frag_source += &*format!("#define ARCTIC {:.1}\n", world::ARCTIC);

        if self.wave_quality != 0 {
            renderer.enable_oes_standard_derivatives();
            background_frag_source += &*format!("#define WAVES {}\n", self.wave_quality * 2);
        }

        background_frag_source += background_frag_template;

        renderer.create_shader(
            include_str!("./shaders/background.vert"),
            &background_frag_source,
        )
    }

    fn prepare(&mut self, renderer: &Renderer, shader: &mut ShaderBinding) {
        let matrix = self.last_view.world_space_to_uv_space();
        shader.uniform_matrix3f("uTexture", &matrix);

        // Time only changes if wave animations are on.
        if self.animations {
            shader.uniform1f("uTime", renderer.time);
        }

        shader.uniform_texture("uSampler", &self.terrain_texture, 0);
        shader.uniform_texture("uGrass", &self.grass_texture, 1);
        shader.uniform_texture("uSand", &self.sand_texture, 2);
        shader.uniform_texture("uSnow", &self.snow_texture, 3);
    }

    fn frame_cache_enabled(&self) -> bool {
        !self.animations
    }

    fn take_invalidation(&mut self) -> Option<Invalidation> {
        std::mem::take(&mut self.invalidation)
    }
}

pub struct Mk48OverlayContext {
    u_above: f32,
    u_area: f32,
    u_border: f32,
    u_restrict: f32,
    u_visual: f32,
}

impl Default for Mk48OverlayContext {
    fn default() -> Self {
        Self {
            u_above: 0.0,
            u_area: 0.0,
            u_border: 1000.0,
            u_restrict: 0.0,
            u_visual: 0.0,
        }
    }
}

impl Mk48OverlayContext {
    pub fn update(
        &mut self,
        visual_range: f32,
        visual_restriction: f32,
        world_radius: f32,
        area: Option<(f32, bool)>,
    ) {
        self.u_visual = visual_range;
        self.u_restrict = visual_restriction;
        self.u_border = world_radius;
        self.u_above = area
            .as_ref()
            .map(|(_, above)| if *above { 1.0 } else { -1.0 })
            .unwrap_or_default();
        self.u_area = area.map(|(area, _)| area).unwrap_or_default()
    }
}

impl BackgroundContext for Mk48OverlayContext {
    fn create_shader(&self, renderer: &mut Renderer) -> Shader {
        renderer.create_shader(
            include_str!("./shaders/overlay.vert"),
            include_str!("./shaders/overlay.frag"),
        )
    }

    fn prepare(&mut self, _: &Renderer, shader: &mut ShaderBinding) {
        shader.uniform3f(
            "uAbove_uArea_uBorder",
            vec3(self.u_above, self.u_area, self.u_border),
        );
        shader.uniform2f("uRestrict_uVisual", vec2(self.u_restrict, self.u_visual));
    }
}

/// Generates trees, coral, etc. for visible terrain.
fn generate_vegetation<'a>(
    terrain_bytes: &'a [u8],
    view: TerrainView,
) -> impl Iterator<Item = SortableSprite> + 'a {
    let center = view.center;
    let width = view.dimensions.x as usize;
    let height = view.dimensions.y as usize;

    // Must be power of 2.
    let step = 2usize.pow(2);
    let step_log2 = (step - 1).count_ones();

    // Round starting coords down to step.
    let start_x = center.0 as isize - (width / 2) as isize & !(step as isize - 1);
    let start_y = center.1 as isize - (height / 2) as isize & !(step as isize - 1);

    // Don't need to round down because step by already handles it.
    let end_x = center.0 as isize + ((width + 1) / 2) as isize;
    let end_y = (center.1 as isize + ((height + 1) / 2) as isize).min(terrain::ARCTIC as isize);

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

            // TODO don't use a fake EntityId.
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
