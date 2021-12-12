// SPDX-FileCopyrightText: 20 21 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::buffer::*;
use crate::deque::PointRenderDeque;
use crate::renderer::Renderer;
use glam::Vec2;
use web_sys::{OesVertexArrayObject as OesVAO, WebGlRenderingContext as Gl};

const LIFESPAN: f32 = 1.25;

/// Particle represents a single particle and contains information about how to update it.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    /// Possible values:
    /// -1 to 1: Fire to black
    ///  0 to 1: Black to white
    pub color: f32,
    pub radius: f32,
    pub created: f32,
}

impl Vertex for Particle {
    fn bind_attribs(attribs: &mut Attribs<Self>) {
        Vec2::bind_attrib(attribs);
        Vec2::bind_attrib(attribs);
        f32::bind_attrib(attribs);
        f32::bind_attrib(attribs);
        f32::bind_attrib(attribs);
    }
}

pub struct ParticleSystem {
    buffer: PointRenderDeque<Particle>,
}

impl ParticleSystem {
    pub fn new(gl: &Gl, oes: &OesVAO) -> Self {
        Self {
            buffer: PointRenderDeque::new(gl, oes),
        }
    }

    /// cannot push particle with p.created < previous particle
    pub fn push(&mut self, p: Particle) {
        self.buffer.push_back(p);
    }

    pub fn expire_particles(&mut self, time_seconds: f32) {
        let expired = time_seconds - LIFESPAN;
        loop {
            if let Some(particle) = self.buffer.front() {
                // Not expired yet (all particles after are must be >= as well).
                if particle.created >= expired {
                    break;
                }
            } else {
                // Queue empty.
                break;
            }
            self.buffer.pop_front();
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer, time: f32, wind: Vec2) {
        if self.buffer.get_buffer().is_empty() {
            return;
        }

        if let Some(mut shader) = renderer
            .particle_shader
            .bind(&renderer.gl, renderer.khr.as_ref())
        {
            shader.uniform_matrix3f("uView", &renderer.view_matrix);
            shader.uniform2f("uWind", wind);
            shader.uniform1f("uTime", time);
            shader.uniform1f("uWindowSize", renderer.canvas.width() as f32 * 0.5);

            self.buffer.buffer(&renderer.gl);

            let buffer = self.buffer.bind(&renderer.gl, &renderer.oes_vao);
            buffer.draw();
        }
    }
}
