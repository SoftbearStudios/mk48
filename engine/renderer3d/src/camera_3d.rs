// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::{Mat4, Vec3};
use renderer::{Camera, Renderer, ShaderBinding};

/// A 3 dimensional camera. It's recommended to create a [new][`Camera3d::new_pitch_yaw`] one each
/// frame.
#[derive(Default)]
pub struct Camera3d {
    /// The location of the [`Camera3d`] in world space.
    pub pos: Vec3,
    /// The [projection matrix](https://en.wikipedia.org/wiki/Projection_matrix).
    pub projection_matrix: Mat4,
    /// The inverse of the [camera matrix](https://en.wikipedia.org/wiki/Camera_matrix).
    pub view_matrix: Mat4,
    /// Equal to `projection_matrix * view_matrix`.
    pub vp_matrix: Mat4,
}

impl Camera for Camera3d {
    /// Sets `uniform mat4 uViewProjection;`.
    fn uniform_matrix(&self, shader: &ShaderBinding) {
        shader.uniform3f("uCameraPos", self.pos);
        shader.uniform_matrix4f("uViewProjection", &self.vp_matrix);
    }

    fn init_render(renderer: &Renderer<Self>) {
        renderer.enable_depth_test();
        renderer.enable_cull_face();
    }
}

/// The data required to create a
/// [projection matrix](https://en.wikipedia.org/wiki/Projection_matrix).
pub struct Projection {
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

impl Default for Projection {
    fn default() -> Self {
        Self {
            aspect: 1.0,
            fov: 80.0,
            z_near: 0.01,
            z_far: 80.0,
        }
    }
}

impl Camera3d {
    /// Creates a new [`Camera3d`] with a mouse controlled `pitch` and `yaw`.
    pub fn new_pitch_yaw(pos: Vec3, pitch: f32, yaw: f32, projection: Projection) -> Self {
        let view_matrix = (Mat4::from_translation(pos)
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_rotation_x(pitch))
        .inverse();

        Self::from_view(pos, view_matrix, projection)
    }

    /// Creates a new [`Camera3d`] that is looking from `pos` at `target`. Assumes up is +Y.
    pub fn new_looking_at(pos: Vec3, target: Vec3, projection: Projection) -> Self {
        let view_matrix = Mat4::look_at_rh(pos, target, Vec3::Y);
        Self::from_view(pos, view_matrix, projection)
    }

    fn from_view(pos: Vec3, view_matrix: Mat4, p: Projection) -> Self {
        let projection_matrix =
            Mat4::perspective_rh_gl(p.fov.to_radians(), p.aspect, p.z_near, p.z_far);
        let vp_matrix = projection_matrix * view_matrix;

        Self {
            pos,
            projection_matrix,
            view_matrix,
            vp_matrix,
        }
    }
}
