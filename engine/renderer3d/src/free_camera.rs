use crate::{Camera3d, Projection};
use glam::{vec3, Vec2, Vec3};
use std::f32::consts::PI;

/// A camera for testing. TODO find a better place than renderer3d.
pub struct FreeCamera {
    pos: Vec3,
    pitch: f32,
    yaw: f32,
    /// The `speed` passed to [`new`][`Self::new`].
    pub speed: f32,
}

impl FreeCamera {
    /// Creates a new [`FreeCamera`] with a given `speed`.
    pub fn new(pos: Vec3, pitch: f32, yaw: f32, speed: f32) -> Self {
        Self {
            pos,
            pitch,
            yaw,
            speed,
        }
    }

    /// Updates the [`FreeCamera`] a mouse movement of `delta_radians`.
    pub fn update_mouse(&mut self, delta_radians: Vec2) {
        self.pitch = (self.pitch + delta_radians.y).clamp(-PI * 0.5, PI * 0.5);
        self.yaw += delta_radians.x; // TODO could normalize.
    }

    /// Updates the [`FreeCamera`] with `keys` pressed and delta time.
    /// `keys` should be in the order: `[Key::D, Key::A, Key::Space, Key::Shift, Key::S, Key::W]`.
    pub fn update_keys(&mut self, keys: [bool; 6], dt: f32) {
        let mut vel = Vec3::ZERO;
        for (v, [up, down]) in vel.as_mut().iter_mut().zip(keys.array_chunks()) {
            *v = if *up && !*down {
                1.0
            } else if *down && !*up {
                -1.0
            } else {
                0.0
            };
        }

        let vel = vel.normalize_or_zero() * self.speed;
        let (s, c) = self.yaw.sin_cos();
        self.pos += vec3(vel.x * c + vel.z * s, vel.y, -vel.x * s + vel.z * c) * dt
    }

    /// Gets the [`Camera3d`] from the [`FreeCamera`].
    pub fn camera_3d(&self, projection: impl Projection) -> Camera3d {
        Camera3d::pitch_yaw(self.pos, self.pitch, self.yaw, projection)
    }
}
