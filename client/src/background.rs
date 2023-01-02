// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Params;
use crate::settings::ShadowSetting;
use crate::sortable_sprite::SortableSprite;
use crate::tessellation::TessellationLayer;
use crate::weather::Weather;
use common::entity::{EntityId, EntityType};
use common::terrain::{Coord, RelativeCoord, Terrain};
use common::transform::Transform;
use common::velocity::Velocity;
use common::{terrain, world};
use common_util::angle::{Angle, AngleRepr};
use glam::{uvec2, vec2, vec3, Mat3, Mat4, Quat, UVec2, Vec2, Vec3};
use renderer::{DefaultRender, Layer, RenderLayer, Renderer, Shader, Texture, TextureFormat};
use renderer2d::{BackgroundLayer, Camera2d, Invalidation, Mask};
use renderer3d::{Aabb3, Camera3d, Orthographic, ShadowParams, ShadowResult};

// Bicubic interpolation in shader needs 4x4 neighbor pixels.
const KERNEL: u32 = 4;
const Z_RANGE: f32 = 400.0;

// Diagonal kernel of 4 pixels.
// _ _ _ X
// _ _ x _
// _ x _ _
// x _ _ _
// X is original pixel, x are generated pixels.
// TODO calculate this from terrain scale and default sun dir (requires const fp math).
const SHADOW_KERNEL: u32 = 4;

#[derive(Copy, Clone, Default, Eq, PartialEq)]
struct TerrainView {
    center: Coord,
    dimensions: UVec2,
}

impl TerrainView {
    fn new(camera: Vec2, aspect: f32, zoom: f32) -> Self {
        // Add 3 for shadow padding (TODO only add in direction of sun).
        const PADDING: u32 = KERNEL / 2 + SHADOW_KERNEL;

        // Both width and height must be odd numbers so there is an equal distance from the center
        // on both sides.
        fn view_width(zoom: f32) -> u32 {
            ((zoom * (1.0 / terrain::SCALE)).ceil() as u32).max(PADDING) * 2 + PADDING * 2 + 1
        }

        Self {
            center: Coord::from_position(camera).unwrap(),
            dimensions: uvec2(view_width(zoom), view_width(zoom / aspect)),
        }
    }

    fn corner(&self) -> Coord {
        Coord::from_uvec2(self.center.as_uvec2() - self.dimensions / 2)
    }

    /// Returns a matrix that translates world space to terrain texture UV coordinates.
    fn world_space_to_uv_space(&self) -> Mat3 {
        let offset = Mat3::from_translation(-self.center.corner());
        let scale = &Mat3::from_scale((self.dimensions.as_vec2() * terrain::SCALE).recip());
        Mat3::from_translation(Vec2::splat(0.5)).mul_mat3(&scale.mul_mat3(&offset))
    }

    fn shadow_model_matrix(&self) -> Mat4 {
        let translation = Mat4::from_translation(self.center.corner().extend(-Z_RANGE * 0.5));
        let scale = Mat4::from_scale((self.dimensions.as_vec2() * terrain::SCALE).extend(Z_RANGE));
        translation
            .mul_mat4(&scale)
            .mul_mat4(&Mat4::from_translation(-Vec2::splat(0.5).extend(0.0)))
    }
}

#[derive(Layer)]
pub struct Mk48BackgroundLayer {
    #[layer]
    background: BackgroundLayer,
    #[layer]
    tesselation: TessellationLayer,
    shader: Shader,
    shadow_shader: Shader,
    height_texture: Texture,
    detail_texture: Texture,
    detail_load: UVec2, // Dimensions to know when detail texture is done loading.
    pub cache_frame: bool,
    last_view: TerrainView,
    last_terrain: Vec<u8>,
    last_vegetation: Vec<SortableSprite>,
    invalidation: Option<Invalidation>,
    shadow_setting: ShadowSetting,
}

impl Mk48BackgroundLayer {
    pub fn new(
        renderer: &Renderer,
        animations: bool,
        dynamic_waves: bool,
        shadow_setting: ShadowSetting,
    ) -> Self {
        let inner = BackgroundLayer::new(renderer);

        let mut defines = format!("#define ARCTIC {:.1}\n", world::ARCTIC);
        if animations {
            defines += "#define ANIMATIONS\n";
        }
        if dynamic_waves {
            // renderer.enable_oes_standard_derivatives();
            defines += "#define WAVES 6\n";
        }
        defines += shadow_setting.shader_define();
        let frag = include_str!("./shaders/background.frag").replace("#defines", &defines);

        // Don't cache shader because it's dynamic.
        let shader = Shader::new(renderer, include_str!("./shaders/background.vert"), &frag);
        let shadow_shader = renderer.create_shader(
            include_str!("shaders/background_shadow.vert"),
            include_str!("shaders/shadow.frag"),
        );

        // Don't interpolate because it's done in the shader.
        let height_texture = Texture::new_empty(renderer, TextureFormat::Alpha, false);

        let detail_texture = Texture::load(
            renderer,
            "/textures.png",
            TextureFormat::COLOR_RGBA_STRAIGHT,
            Some([184, 73, 235]),
            true,
        );
        let detail_load = detail_texture.dimensions();

        Mk48BackgroundLayer {
            background: inner,
            cache_frame: !animations,
            detail_load,
            detail_texture,
            height_texture,
            invalidation: None,
            last_terrain: vec![],
            last_vegetation: vec![],
            last_view: TerrainView::default(),
            shader,
            shadow_setting,
            shadow_shader,
            tesselation: TessellationLayer::new(renderer),
        }
    }

    // Update the background with terrain.
    // TODO don't rely on mutating terrain to get updates (compare with last known state).
    pub fn update(
        &mut self,
        camera: Vec2,
        zoom: f32,
        terrain: &mut Terrain,
        terrain_reset: bool,
        has_shadows: bool,
        renderer: &Renderer,
    ) -> impl Iterator<Item = SortableSprite> + '_ {
        let view = TerrainView::new(camera, renderer.aspect_ratio(), zoom);
        let view_changed = view != self.last_view;

        // TODO Only if update happened in our current view.
        let terrain_changed = !terrain.updated.is_empty() || terrain_reset;

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

            self.height_texture.realloc_with_opt_bytes(
                renderer,
                view.dimensions,
                Some(&self.last_terrain),
            );

            // Vegetation only changes if any of its arguments change.
            // Reuse previous allocation.
            self.last_vegetation.clear();
            self.last_vegetation
                .extend(generate_vegetation(&self.last_terrain, view))
        }

        // Determine when the detail texture loads (when its dimensions change) so we can invalidate
        // the bg. This trick won't work on 1x1 textures. TODO find a better method.
        let detail_dim = self.detail_texture.dimensions();
        let detail_just_loaded = self.detail_load != detail_dim;
        self.detail_load = detail_dim;

        // Only create invalidations if frame cache is enabled and they'll be used.
        if self.cache_frame {
            // Invalidate bg when terrain is reset (aka switch servers) or when detail texture loads.
            if terrain_reset || detail_just_loaded {
                self.invalidation = Some(Invalidation::All);
            } else if !terrain.updated.is_empty() {
                let updated = terrain.updated.clone();
                let dimensions = view.dimensions;
                let corner = view.corner().as_uvec2();

                // Invalidate a square around each point (for shader interpolation).
                let mask = Mask::new_expanded(
                    updated
                        .into_iter()
                        .flat_map(|chunk_id| {
                            terrain
                                .get_chunk(chunk_id)
                                .updated_coords(chunk_id) // TODO whole chuck mask of points is inefficient.
                                .map(|coord| {
                                    (coord.as_uvec2().as_ivec2() - corner.as_ivec2()).as_uvec2()
                                })
                        })
                        .flat_map(|c| {
                            let diagonal_count = if has_shadows { SHADOW_KERNEL } else { 1 };

                            // TODO diagonal prob only needs 2x2 around each pixel.
                            // TODO could also limit diagonal count based on pixel height relative to water.
                            // TODO don't depend on the sun being in this exact direction.
                            (0..diagonal_count).filter_map(move |i| {
                                let coord = (c.as_ivec2() - i as i32).as_uvec2();
                                (coord.cmple(dimensions).all()).then_some(coord)
                            })
                        }),
                    dimensions,
                    KERNEL,
                );

                let rects: Vec<_> = mask
                    .into_rects()
                    .map(|(start, end)| {
                        let start = Coord::from_uvec2(start + corner);
                        let end = Coord::from_uvec2(end + corner);

                        let end = end + RelativeCoord(1, 1);
                        let offset = -terrain::SCALE * 1.0;

                        let s = start.corner() + offset;
                        let e = end.corner() + offset;

                        (s, e)
                    })
                    .collect();

                // TODO check is mask is empty before creating rects.
                if !rects.is_empty() {
                    self.invalidation = Some(Invalidation::Rects(rects));
                }
            }
        }

        // Finish updates.
        terrain.clear_updated();
        self.last_view = view;

        self.last_vegetation.iter().copied()
    }

    pub fn shadow_camera(
        &self,
        renderer: &Renderer,
        camera: &Camera2d,
        weather: &Weather,
        shadows: ShadowSetting,
    ) -> Camera3d {
        if shadows.is_none() {
            return Default::default();
        }

        let center = camera.center;
        let width = camera.zoom * 2.0;
        let aspect = renderer.aspect_ratio();

        let mut center = center.extend(0.0);
        let mut dimensions = vec3(width, width / aspect, Z_RANGE);

        // Pad shadow map to reduce snapping.
        // TODO look into fixing visible snapping every 100 meters of movement.
        // TODO variable shadow map resolution based on viewport so softness changes with zoom.
        let scale = terrain::SCALE * 4.0;
        let inv_scale = 1.0 / scale;

        // Round to scale.
        center = (center * inv_scale).round() * scale;

        // Round up to nearest 2 scale.
        dimensions = (dimensions * inv_scale * 0.5).ceil() * scale * 2.0;
        let aabb = Aabb3::from_center_and_dimensions(center, dimensions);

        // TODO figure out why it needs this factor.
        let sun = weather.sun * vec3(-1.0, -1.0, 1.0);
        let rotation = Mat4::from_quat(Quat::from_rotation_arc(Vec3::Z, sun));
        let aabb = aabb.transformed_by(&rotation);

        let pos = ((aabb.min.truncate() + aabb.max.truncate()) * 0.5).extend(aabb.max.z);
        let target = pos - Vec3::Z;
        let dimensions = aabb.max - aabb.min;

        let inv_rot = rotation.inverse();
        let pos = inv_rot.transform_point3(pos);
        let target = inv_rot.transform_point3(target);

        let projection = Orthographic { dimensions };
        Camera3d::looking_at(pos, target, projection)
    }
}

impl RenderLayer<&ShadowResult<&Mk48Params>> for Mk48BackgroundLayer {
    fn render(&mut self, renderer: &Renderer, result: &ShadowResult<&Mk48Params>) {
        // TODO don't clear shadow map and redraw shadows if inputs haven't changed.

        if let Some(shader) = self.shader.bind(renderer) {
            renderer.set_blend(false); // Optimization, doesn't require blend.

            result.prepare_shadows(&shader);
            let params = &result.params;

            shader.uniform("uTexture", &self.last_view.world_space_to_uv_space());
            shader.uniform("uDerivative", params.camera.derivative());

            // Only use weather/time if we aren't caching the frame.
            let mut weather = Weather::default();
            if !self.cache_frame {
                weather = params.weather;
                shader.uniform("uTime", renderer.time);
                shader.uniform("uWind", weather.wind);
            }
            shader.uniform("uSun", weather.sun);
            shader.uniform("uWaterSun", weather.water_sun());

            shader.uniform("uHeight", &self.height_texture);
            shader.uniform("uDetail", &self.detail_texture);

            self.background.render(
                renderer,
                (
                    shader,
                    &params.camera,
                    self.cache_frame.then(|| self.invalidation.take()),
                ),
            );

            renderer.set_blend(true);
        }
    }
}

impl RenderLayer<&ShadowParams> for Mk48BackgroundLayer {
    fn render(&mut self, renderer: &Renderer, params: &ShadowParams) {
        // TODO depth layer.
        renderer.set_depth_test(true);

        if let Some(shader) = self.shadow_shader.bind(renderer) {
            params.camera.prepare_without_camera_pos(&shader);

            shader.uniform("uModel", &self.last_view.shadow_model_matrix());
            shader.uniform("uHeight", &self.height_texture);

            // TODO account for aspect ratio instead of hardcoding 16 by 9.
            let soft = self.shadow_setting == ShadowSetting::Soft;
            let animations = !self.cache_frame;

            // Use lower vertex count for soft shadows because the shadow map is lower resolution.
            // Animated zoom makes soft jitter too much with low vertex count.
            let dim = if soft && !animations {
                uvec2(192, 108) // Soft shadow map is 1/4 res so use dim / 4.
            } else {
                uvec2(192, 108) * 4
            };

            self.tesselation.render(renderer, (&shader, dim));
        }

        // TODO depth layer.
        renderer.set_depth_test(false);
    }
}

#[derive(Layer)]
#[alpha]
pub struct Mk48OverlayLayer {
    #[layer]
    inner: BackgroundLayer,
    shader: Shader,
    u_above: f32,
    u_area: f32,
    u_border: f32,
    u_restrict: f32,
    u_visual: f32,
}

impl DefaultRender for Mk48OverlayLayer {
    fn new(renderer: &Renderer) -> Self {
        let inner = BackgroundLayer::new(renderer);
        let shader = renderer.create_shader(
            include_str!("./shaders/overlay.vert"),
            include_str!("./shaders/overlay.frag"),
        );

        Self {
            inner,
            shader,
            u_above: 0.0,
            u_area: 0.0,
            u_border: 1000.0,
            u_restrict: 0.0,
            u_visual: 0.0,
        }
    }
}

impl Mk48OverlayLayer {
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

impl RenderLayer<&Camera2d> for Mk48OverlayLayer {
    fn render(&mut self, renderer: &Renderer, camera: &Camera2d) {
        let shader = self.shader.bind(renderer);
        if let Some(shader) = shader {
            shader.uniform("uMiddle", camera.center);
            shader.uniform(
                "uAbove_uArea_uBorder",
                vec3(self.u_above, self.u_area, self.u_border),
            );
            shader.uniform("uRestrict_uVisual", vec2(self.u_restrict, self.u_visual));

            self.inner.render(renderer, (shader, camera, None));
        }
    }
}

/// Generates trees, coral, etc. for visible terrain.
fn generate_vegetation(
    terrain_bytes: &[u8],
    view: TerrainView,
) -> impl Iterator<Item = SortableSprite> + '_ {
    let center = view.center;
    let width = view.dimensions.x as usize;
    let height = view.dimensions.y as usize;

    // Must be power of 2.
    let step = 2usize.pow(2);
    let step_log2 = (step - 1).count_ones();

    // Round starting coords down to step.
    let start_x = (center.0 as isize - (width / 2) as isize) & !(step as isize - 1);
    let start_y = (center.1 as isize - (height / 2) as isize) & !(step as isize - 1);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_kernel() {
        let weather = Weather::default();
        let sun = weather.sun;
        let height = 100.0; // Water to tallest mountain.
        let mul = height / sun.z;
        let len = sun.truncate() * mul;

        let u = (len / terrain::SCALE).ceil().as_uvec2() + 1;
        println!("requires {u} units");
        assert_eq!(u, UVec2::splat(SHADOW_KERNEL), "change SHADOW_KERNEL const");
    }
}
