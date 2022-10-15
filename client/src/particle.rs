// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use glam::Vec2;
use renderer::{derive_vertex, LayerShader, Shader, ShaderBinding};
use renderer2d::{Camera2d, Particle, ParticleContext, ParticleLayer, Renderer2d};

derive_vertex!(
    pub struct Mk48Particle {
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
);
impl Particle for Mk48Particle {
    const LIFESPAN: f32 = 1.25;
}

pub type Mk48ParticleLayer = ParticleLayer<Mk48ParticleContext>;
pub struct Mk48ParticleContext {
    pub wind: Vec2,
}

impl LayerShader<Camera2d> for Mk48ParticleContext {
    fn create(&self, renderer: &Renderer2d) -> Shader {
        renderer.create_shader(
            include_str!("shaders/particle.vert"),
            include_str!("shaders/particle.frag"),
        )
    }

    fn prepare(&mut self, renderer: &Renderer2d, shader: &ShaderBinding) {
        let width = renderer.camera.pixels_per_unit();
        shader.uniform4f(
            "uWind_uTime_uScale",
            self.wind.extend(renderer.time).extend(width),
        );
    }
}

impl ParticleContext for Mk48ParticleContext {
    type Particle = Mk48Particle;
}
