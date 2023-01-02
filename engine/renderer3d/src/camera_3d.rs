// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{Mat4, Vec3};
use renderer::{Renderer, ShaderBinding};

/// A 3 dimensional camera. It's recommended to create a new one each frame.
#[derive(Clone, Default)]
pub struct Camera3d {
    /// The [camera matrix](https://en.wikipedia.org/wiki/Camera_matrix).
    pub camera_matrix: Mat4,
    /// The [projection matrix](https://en.wikipedia.org/wiki/Projection_matrix).
    pub projection_matrix: Mat4,
    /// The [`inverse`][`Mat4::inverse`] of `camera_matrix`.
    pub view_matrix: Mat4,
    /// Equal to `projection_matrix * view_matrix`.
    pub vp_matrix: Mat4,
}

impl Camera3d {
    /// Sets `uniform vec3 uCameraPos;` and `uniform mat4 uViewProjection;`.
    pub fn prepare(&self, shader: &ShaderBinding) {
        self.debug_assert_valid();
        shader.uniform("uCameraPos", self.position());
        shader.uniform("uViewProjection", &self.vp_matrix);
    }

    /// Only sets `uniform mat4 uViewProjection;`.
    #[doc(hidden)]
    pub fn prepare_without_camera_pos(&self, shader: &ShaderBinding) {
        self.debug_assert_valid();
        shader.uniform("uViewProjection", &self.vp_matrix);
    }

    /// TODO remove this hack for Camera3d and replace with DepthLayer.
    pub fn init(renderer: &Renderer) {
        renderer.set_depth_test(true);
        renderer.enable_cull_face();
    }

    fn debug_assert_valid(&self) {
        #[cfg(debug_assertions)]
        if self.vp_matrix == Mat4::default() {
            js_hooks::console_log!("using invalid Camera3d")
        }
    }
}

/// [`Perspective`] projections make objects that are further from the [`Camera3d`] appear smaller.
pub struct Perspective {
    /// [Aspect ratio](https://en.wikipedia.org/wiki/Aspect_ratio_(image)) of viewport (get with
    /// [`Renderer::aspect_ratio`][`renderer::Renderer::aspect_ratio`]). Required
    /// or defaults to `1.0`.
    pub aspect: f32,
    /// [Field of view](https://en.wikipedia.org/wiki/Field_of_view) in degrees. Defaults to `80.0`.
    pub fov: f32,
    /// Near [clip](https://en.wikipedia.org/wiki/Clipping_(computer_graphics)) plane in world
    /// space. Defaults to `0.01`.
    pub z_near: f32,
    /// Far [clip](https://en.wikipedia.org/wiki/Clipping_(computer_graphics)) plane in world space.
    /// Defaults to `80.0`.
    pub z_far: f32,
}

impl Default for Perspective {
    fn default() -> Self {
        Self {
            aspect: 1.0,
            fov: 80.0,
            z_near: 0.01,
            z_far: 80.0,
        }
    }
}

impl Projection for Perspective {
    fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov.to_radians(), self.aspect, self.z_near, self.z_far)
    }
}

/// [`Orthographic`] projections make objects always appear the same size no matter the distance
/// from the [`Camera3d`].
pub struct Orthographic {
    /// The dimensions that the projection covers.
    pub dimensions: Vec3,
}

impl Projection for Orthographic {
    fn projection_matrix(&self) -> Mat4 {
        let start = self.dimensions.truncate() * -0.5;
        let end = -start;
        Mat4::orthographic_rh_gl(start.x, end.x, start.y, end.y, 0.0, self.dimensions.z)
    }
}

/// A [`Projection`] defines how objects appear based on their distance to the [`Camera3d`].
pub trait Projection {
    /// Gets the [projection matrix](https://en.wikipedia.org/wiki/Projection_matrix).
    fn projection_matrix(&self) -> Mat4;
}

impl Camera3d {
    /// Creates a new [`Camera3d`] with a mouse controlled `pitch` and `yaw`.
    pub fn pitch_yaw(pos: Vec3, pitch: f32, yaw: f32, projection: impl Projection) -> Self {
        let camera_matrix =
            Mat4::from_translation(pos) * Mat4::from_rotation_y(yaw) * Mat4::from_rotation_x(pitch);

        let view_matrix = camera_matrix.inverse();
        Self::from_camera_and_view(camera_matrix, view_matrix, projection)
    }

    /// Creates a new [`Camera3d`] that is looking from `pos` at `target`. Assumes up is +Y.
    pub fn looking_at(pos: Vec3, target: Vec3, projection: impl Projection) -> Self {
        let view_matrix = Mat4::look_at_rh(pos, target, Vec3::Y);
        Self::with_view(view_matrix, projection)
    }

    /// Creates a new [`Camera3d`] with a `view_matrix`.
    #[doc(hidden)]
    pub fn with_view(view_matrix: Mat4, projection: impl Projection) -> Self {
        let camera_matrix = view_matrix.inverse();
        Self::from_camera_and_view(camera_matrix, view_matrix, projection)
    }

    fn from_camera_and_view(
        camera_matrix: Mat4,
        view_matrix: Mat4,
        projection: impl Projection,
    ) -> Self {
        let projection_matrix = projection.projection_matrix();
        let vp_matrix = projection_matrix * view_matrix;

        Self {
            camera_matrix,
            projection_matrix,
            view_matrix,
            vp_matrix,
        }
    }

    /// Returns the position of the [`Camera3d`].
    pub(crate) fn position(&self) -> Vec3 {
        self.camera_matrix.w_axis.truncate()
    }

    /// Returns the normal aka the direction the [`Camera3d`] is looking.
    pub(crate) fn normal(&self) -> Vec3 {
        -self.camera_matrix.z_axis.truncate()
    }

    /// Projects from world space to ndc space.
    pub(crate) fn world_to_ndc(&self, pos: Vec3) -> Vec3 {
        self.vp_matrix.project_point3(pos)
    }
}
