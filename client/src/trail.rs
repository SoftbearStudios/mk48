// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Params;
use crate::weather::Weather;
use common::entity::EntityId;
use common::ticks::Ticks;
use common_util::range::map_ranges;
use glam::{Vec2, Vec3, Vec4};
use itertools::Itertools;
use renderer::{DefaultRender, Layer, RenderLayer, Renderer};
use renderer2d::GraphicLayer;
use std::cmp::Ordering;
use std::collections::HashMap;

struct Trail {
    created: f32,
    end: Vec2,
    lifespan: f32,
    start: Vec2,
    updated: f32,
    width: f32,
}

impl Trail {
    fn new(pos: Vec2, width: f32, lifespan: f32, time: f32) -> Trail {
        Trail {
            created: time,
            end: pos,
            lifespan,
            start: pos,
            updated: time,
            width,
        }
    }

    fn update(&mut self, pos: Vec2, time: f32) {
        self.end = pos;
        self.updated = time;
    }

    fn add_to_layer(&self, layer: &mut GraphicLayer, weather: &Weather, time: f32) {
        // How long the start point of the trail has been visible.
        // Clamp the start point to the visible range.
        let start_alive = time - self.created;
        let start_clamp = (start_alive - self.lifespan).max(0.0);
        let start_alive = start_alive.min(self.lifespan);

        // Move start position towards end position based on how much start alive was clamped.
        let start_pos = self
            .start
            .lerp(self.end, start_clamp / (self.updated - self.created))
            + Self::offset(weather, start_alive);
        let start_color = self.color(start_alive);

        // How long the end point of the trail has been visible.
        // Don't need to clamp the end point because it will be expired first.
        let end_alive = time - self.updated;
        let end_pos = self.end + Self::offset(weather, end_alive);
        let end_color = self.color(end_alive);

        layer.draw_line_gradient(start_pos, end_pos, self.width, start_color, end_color);
    }

    fn expired(&self, time: f32) -> bool {
        self.updated < time - self.lifespan
    }

    fn color(&self, alive: f32) -> Vec4 {
        debug_assert!(
            alive >= 0.0 && alive <= self.lifespan,
            "{alive}, {}",
            self.lifespan
        );
        Vec3::ONE.extend(map_ranges(alive, 0.0..self.lifespan, 0.05..0.0, false))
    }

    fn offset(weather: &Weather, alive: f32) -> Vec2 {
        weather.wind * alive
    }
}

impl PartialEq<Self> for Trail {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).map(|o| o.is_eq()).unwrap_or(false)
    }
}

impl Eq for Trail {}

impl PartialOrd for Trail {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Trail {
    fn cmp(&self, other: &Self) -> Ordering {
        self.created
            .partial_cmp(&other.created)
            .unwrap()
            .then_with(|| {
                self.start
                    .x
                    .partial_cmp(&other.start.x)
                    .unwrap()
                    .then_with(|| self.start.y.partial_cmp(&other.start.y).unwrap())
            })
    }
}

#[derive(Layer)]
pub struct TrailLayer {
    #[layer]
    inner: GraphicLayer,
    time: f32,
    trails: HashMap<EntityId, Trail>,
    unowned_trails: Vec<Trail>,
}

impl DefaultRender for TrailLayer {
    fn new(renderer: &Renderer) -> Self {
        // TODO implement #[derive(DefaultRender)]
        Self {
            inner: DefaultRender::new(renderer),
            time: Default::default(),
            trails: Default::default(),
            unowned_trails: Default::default(),
        }
    }
}

impl TrailLayer {
    pub fn set_time(&mut self, time: f32) {
        self.time = time;
    }

    pub fn add_trail(&mut self, id: EntityId, pos: Vec2, vel: Vec2, width: f32) {
        let time = self.time;
        self.trails
            .entry(id)
            .or_insert_with(|| {
                // Move trail back 1 tick.
                let delta = -Ticks::ONE.to_secs();

                Trail::new(pos + vel * delta, width, 1.0, time + delta)
            })
            .update(pos, self.time);
    }
}

impl RenderLayer<&Mk48Params> for TrailLayer {
    fn render(&mut self, renderer: &Renderer, params: &Mk48Params) {
        let time = self.time;

        // Move trails that weren't updated.
        self.unowned_trails.extend(
            self.trails
                .drain_filter(|_, t| t.updated != time)
                .map(|(_, t)| t),
        );

        // Drain expired trails.
        self.unowned_trails.drain_filter(|t| t.expired(time));

        // Add owned and unowned trails to graphics layer.
        // TODO sorted because alpha blending isn't additive (yet).
        for trail in self
            .trails
            .values()
            .chain(self.unowned_trails.iter())
            .sorted_unstable()
        {
            trail.add_to_layer(&mut self.inner, &params.weather, time)
        }

        self.inner.render(renderer, &params.camera);
    }
}
