// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Params;
use crate::settings::ShadowSetting;
use glam::Vec2;
use renderer::{derive_vertex, DefaultRender, Layer, RenderLayer, Renderer, Shader};
use renderer2d::{Particle, ParticleLayer};
use renderer3d::ShadowResult;
use std::ops::{Deref, DerefMut};

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

/// [`Deref`]s to its inner [`ParticleLayer`].
#[derive(Layer)]
#[alpha]
pub struct Mk48ParticleLayer<const AIRBORNE: bool> {
    #[layer]
    inner: ParticleLayer<Mk48Particle>,
    shader: Shader,
}

impl<const A: bool> Deref for Mk48ParticleLayer<A> {
    type Target = ParticleLayer<Mk48Particle>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<const A: bool> DerefMut for Mk48ParticleLayer<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<const A: bool> Mk48ParticleLayer<A> {
    pub fn new(renderer: &Renderer, shadows: ShadowSetting) -> Self {
        let inner = ParticleLayer::new(renderer);

        let mut vert = "#version 300 es\n".to_owned();
        vert += shadows.shader_define();
        vert += include_str!("shaders/particle.vert");

        // TODO don't create 2 shaders for 2 particle layers.
        let shader = Shader::new(renderer, &vert, include_str!("shaders/particle.frag"));

        Self { inner, shader }
    }
}

impl<const A: bool> RenderLayer<&ShadowResult<&Mk48Params>> for Mk48ParticleLayer<A> {
    fn render(&mut self, renderer: &Renderer, result: &ShadowResult<&Mk48Params>) {
        if let Some(shader) = self.shader.bind(renderer) {
            if result.prepare_shadows(&shader) {
                let altitude = if A { 30.0 } else { 0.0 };
                shader.uniform("altitude", altitude);
            }

            let params = &result.params;
            params.camera.prepare(&shader);

            let wind = A.then_some(params.weather.wind).unwrap_or_default();
            let time = renderer.time;
            let width = params.camera.pixels_per_unit();
            shader.uniform("uWind_uTime_uScale", wind.extend(time).extend(width));

            self.inner.render(renderer, &shader);
        }
    }
}
