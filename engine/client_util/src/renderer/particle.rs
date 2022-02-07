// SPDX-FileCopyrightText: 20 21 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::renderer::deque::PointRenderDeque;
use crate::renderer::renderer::Layer;
use crate::renderer::renderer::Renderer;
use crate::renderer::shader::Shader;
use crate::renderer::vertex::Vertex;
use glam::{vec3, vec4, Vec2};

/// Particle represents a single particle and contains information about how to update it.
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    /// Possible values:
    /// -1 to 1: Fire to black
    ///  0 to 1: Black to white
    pub color: f32,
    /// Radius in meters (TODO: make sure actually is in meters).
    pub radius: f32,
    /// 0 = sharp and stays same size, 1 = smooth and gradually dilutes/expands.
    pub smoothness: f32,
}

impl Particle {
    pub const LIFESPAN: f32 = 1.25;
}

/// Renders particles.
pub struct ParticleLayer {
    buffer: PointRenderDeque<ParticleVertex>,
    time: f32,
    wind: Vec2,
}

impl ParticleLayer {
    /// Crates a new particle layer. Wind is the acceleration to apply to particles.
    pub fn new(renderer: &mut Renderer, wind: Vec2) -> Self {
        let gl = &renderer.gl;
        renderer.particle_shader.get_or_insert_with(|| {
            Shader::new(
                gl,
                include_str!("./shaders/particle.vert"),
                include_str!("./shaders/particle.frag"),
            )
        });

        Self {
            buffer: PointRenderDeque::new(&renderer.gl, &renderer.oes_vao),
            time: 0.0,
            wind,
        }
    }

    /// Adds a particle. Once a particle is added, it can be forgotten, as the remainder of its existence
    /// will be managed by this layer.
    pub fn add(&mut self, p: Particle) {
        let Particle {
            color,
            position,
            radius,
            smoothness,
            velocity,
        } = p;

        self.buffer.push_back(ParticleVertex {
            color,
            created: self.time,
            position,
            radius,
            smoothness,
            velocity,
        });
    }
}

/// Particle representation for GPU.
#[derive(Copy, Clone)]
#[repr(C)]
#[derive(Vertex)]
struct ParticleVertex {
    pub position: Vec2,
    pub velocity: Vec2,
    pub color: f32,
    pub radius: f32,
    pub smoothness: f32,
    pub created: f32,
}

impl Layer for ParticleLayer {
    fn pre_prepare(&mut self, r: &Renderer) {
        self.time = r.time;
    }

    fn pre_render(&mut self, renderer: &Renderer) {
        let expired = renderer.time - Particle::LIFESPAN;

        while let Some(particle) = self.buffer.front() {
            // Not expired yet (all particles after are must be >= as well).
            if particle.created >= expired {
                break;
            }
            self.buffer.pop_front();
        }
    }

    fn render(&mut self, renderer: &Renderer) {
        if self.buffer.get_buffer().is_empty() {
            return;
        }

        if let Some(shader) = renderer.bind_shader(renderer.particle_shader.as_ref().unwrap()) {
            let width = renderer.canvas_size().x as f32
                * 0.5
                * (renderer.camera.view_matrix * vec3(1.0, 0.0, 0.0)).length();

            shader.uniform_matrix3f("uView", &renderer.camera.view_matrix);
            shader.uniform4f(
                "uWind_uTime_uScale",
                vec4(self.wind.x, self.wind.y, renderer.time, width),
            );

            self.buffer.buffer(&renderer.gl);
            self.buffer.bind(&renderer.gl, &renderer.oes_vao).draw();
        }
    }
}
