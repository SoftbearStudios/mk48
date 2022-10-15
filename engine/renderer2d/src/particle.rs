// SPDX-FileCopyrightText: 20 21 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::camera_2d::Camera2d;
use crate::Renderer2d;
use bytemuck::{Pod, Zeroable};
use renderer::{Camera, Layer, LayerShader, PointDeque, Shader, Vertex};

/// A single particle (appended with `created: f32`). Requires calling the
/// [`derive_vertex`][`renderer::derive_vertex`] macro.
pub trait Particle: Copy + Clone + Pod + Zeroable + Vertex {
    /// How long the particle will be alive for in seconds. Will be alive for 1 frame minimum.
    const LIFESPAN: f32;
}

/// Implements [`LayerShader<Camera2d>`] and provides an implementation of [`Particle`].
pub trait ParticleContext: LayerShader<Camera2d> {
    /// The type of particle that the [`ParticleLayer`] will draw.
    type Particle: Particle;
}

/// Can't use derive_vertex because not deriving [`Pod`].
#[derive(Copy, Clone, Vertex, Zeroable)]
#[repr(C)]
struct ParticleVertex<T: Vertex + Copy + Pod + Zeroable + 'static> {
    inner: T,
    created: f32,
}

/// Draws point [`Particle`]s.
pub struct ParticleLayer<X: ParticleContext> {
    buffer: PointDeque<ParticleVertex<X::Particle>>,
    /// The [`ParticleContext`] passed to [`new`][`Self::new`].
    pub context: X,
    shader: Shader,
    time: f32,
}

impl<X: ParticleContext> ParticleLayer<X> {
    /// Crates a new [`ParticleLayer`].
    pub fn new(renderer: &Renderer2d, context: X) -> Self {
        let shader = context.create(renderer);
        Self {
            buffer: PointDeque::new(renderer),
            context,
            shader,
            time: 0.0,
        }
    }

    /// Adds a particle. The particle will stay alive for its [`LIFESPAN`][`Particle::LIFESPAN`].
    pub fn add(&mut self, p: X::Particle) {
        self.buffer.push_back(ParticleVertex {
            inner: p,
            created: self.time,
        });
    }
}

impl<X: ParticleContext> Layer<Camera2d> for ParticleLayer<X> {
    fn pre_prepare(&mut self, r: &Renderer2d) {
        self.time = r.time;

        // Expire particles that were created before expiry time.
        let expiry = r.time - X::Particle::LIFESPAN;
        while let Some(particle) = self.buffer.front() && particle.created < expiry {
            self.buffer.pop_front();
        }
    }

    fn render(&mut self, renderer: &Renderer2d) {
        // Ensure ParticleVertex safely implements Pod.
        assert_safe::<X::Particle>();

        if self.buffer.is_empty() {
            return;
        }

        if let Some(shader) = self.shader.bind(renderer) {
            renderer.camera.uniform_matrix(&shader);
            self.context.prepare(renderer, &shader);
            self.buffer.bind(renderer).draw();
        }
    }
}

/// This works around the fact that bytemuck can't derive Pod on non-packed generic structs related
/// issue: https://github.com/Lokathor/bytemuck/issues/7
///
/// Safetey
///
/// Must call [`assert_safe`] before using [`Pod`] on [`ParticleVertex`].
unsafe impl<T: Vertex + Copy + Pod + Zeroable + 'static> Pod for ParticleVertex<T> {}

/// Must call before using [`Pod`] on [`ParticleVertex`].
fn assert_safe<T: Vertex + Copy + Pod + Zeroable + 'static>() {
    // Ensure that ParticleVertex has no padding (so we don't read uninitialized memory).
    assert!(std::mem::align_of::<T>() == 4, "alignment must be 4");
    // Should always be true if the above assert is, but just in case.
    assert!(std::mem::align_of::<ParticleVertex<T>>() == 4);
}
