// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{vec2, IVec2, Mat3, UVec2, Vec2};
use renderer::viewport_to_aspect;
use renderer::ShaderBinding;

/// A 2 dimensional camera.
#[derive(Clone, Default)]
pub struct Camera2d {
    /// The [camera matrix](https://en.wikipedia.org/wiki/Camera_matrix).
    pub camera_matrix: Mat3,
    /// The center of the [`Camera2d`]'s view in world space.
    pub center: Vec2,
    /// The inverse of the [camera matrix](https://en.wikipedia.org/wiki/Camera_matrix).
    pub view_matrix: Mat3,
    /// The width and height in pixels of the screen.
    pub viewport: UVec2,
    /// The width of the [`Camera2d`]'s view in world space.
    pub zoom: f32,
    pub(crate) aligned: Camera2dAligned,
}

impl Camera2d {
    /// Sets `uniform mat3 uView;`.
    pub fn prepare(&self, shader: &ShaderBinding) {
        shader.uniform("uView", &self.view_matrix);
    }
}

impl Camera2d {
    /// Updates the [`Camera2d`] with a `center`, `zoom`, and `viewport`. Get `viewport` from
    /// [`Renderer::canvas_size`][`renderer::Renderer::canvas_size`]
    pub fn update(&mut self, center: Vec2, zoom: f32, viewport: UVec2) {
        let aspect = viewport_to_aspect(viewport);
        let View {
            camera_matrix,
            center,
            view_matrix,
        } = View::new(center, zoom, aspect);

        // Scale changing only happens when either the width or height of the camera changes.
        // This causes an invalidation of delta pixels. In the future this kind of transformation
        // could be captured and used in background.rs.
        let scale_changed = zoom != self.zoom || viewport != self.viewport;
        let aligned = self
            .aligned
            .updated(center, zoom, aspect, viewport, scale_changed);

        // Recreate self to ensure all fields change.
        *self = Self {
            camera_matrix,
            center,
            view_matrix,
            viewport,
            zoom,
            aligned,
        }
    }

    /// Convert a position in view space (`-1.0..1.0`) to world space.
    pub fn to_world_position(&self, view_position: Vec2) -> Vec2 {
        self.camera_matrix.transform_point2(view_position)
    }

    /// Convert a position in world space to view space (`-1.0..1.0`).
    pub fn to_view_position(&self, world_position: Vec2) -> Vec2 {
        self.view_matrix.transform_point2(world_position)
    }

    /// 1 world space unit in pixels.
    pub fn pixels_per_unit(&self) -> f32 {
        let viewport = self.viewport.as_vec2();
        let scale = viewport_scale(viewport, self.zoom);
        (viewport / scale).x // same in x and y
    }

    /// Per pixel derivative in world space (x or y).
    pub fn derivative(&self) -> f32 {
        let matrix = &self.camera_matrix;
        let viewport_meters = (matrix.transform_point2(Vec2::splat(1.0))
            - matrix.transform_point2(Vec2::splat(-1.0)))
        .abs();
        (viewport_meters / self.viewport.as_vec2()).x // same in x and y
    }

    /// Returns the subpixel difference between the aligned camera and the unaligned camera.
    /// Useful for reversing camera alignment.
    pub(crate) fn subpixel_uv_diff(&self) -> Vec2 {
        self.aligned
            .view_matrix
            .transform_vector2(self.aligned.center - self.center)
    }
}

/// Camera that is aligned to pixels. Derefs to a [`Camera2dInner`].
#[derive(Clone, Default)]
pub(crate) struct Camera2dAligned {
    pub(crate) camera_matrix: Mat3,
    pub(crate) center: Vec2,
    pub(crate) delta_pixels: Option<IVec2>, // None = invalidated, Some = moved exact pixels.
    pub(crate) view_matrix: Mat3,
}

impl Camera2dAligned {
    fn updated(
        &self,
        center: Vec2,
        zoom: f32,
        aspect: f32,
        viewport: UVec2,
        scale_changed: bool,
    ) -> Self {
        let (center, delta) = round_to_pixel(center, self.center, zoom, viewport);
        let delta_pixels = (!scale_changed).then_some(delta);

        let View {
            camera_matrix,
            center,
            view_matrix,
        } = View::new(center, zoom, aspect);

        // Recreate Self to ensure all fields change.
        Self {
            camera_matrix,
            center,
            delta_pixels,
            view_matrix,
        }
    }
}

/// The logic of Camera2d that is shared between aligned and unaligned cameras.
struct View {
    camera_matrix: Mat3,
    center: Vec2,
    view_matrix: Mat3,
}

impl View {
    fn new(center: Vec2, zoom: f32, aspect: f32) -> Self {
        // This matrix is the camera matrix manually inverted.
        let view_matrix =
            Mat3::from_scale(vec2(1.0, aspect) / zoom).mul_mat3(&Mat3::from_translation(-center));
        let camera_matrix = view_matrix.inverse();
        Self {
            camera_matrix,
            center,
            view_matrix,
        }
    }
}

/// Useful for calculating meters per pixel adn pixels per meter.
fn viewport_scale(viewport: Vec2, zoom: f32) -> Vec2 {
    // Viewport is -1 to 1, so must double.
    vec2(zoom, zoom * viewport.y / viewport.x) * 2.0
}

fn round_to_pixel(mut pos: Vec2, prev: Vec2, zoom: f32, viewport: UVec2) -> (Vec2, IVec2) {
    let viewport = viewport.as_vec2();
    let scale = viewport_scale(viewport, zoom);

    // Meters per pixel and pixels per meter.
    let mpp = scale / viewport;
    let ppm = viewport / scale;

    // Round center to nearest pixel.
    pos = (pos * ppm).round() * mpp;

    let deltaf = (pos - prev) * ppm;
    let delta = deltaf.round().as_ivec2();

    (pos, delta)
}
