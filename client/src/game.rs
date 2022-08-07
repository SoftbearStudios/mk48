// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::armament::FireRateLimiter;
use crate::audio::Audio;
use crate::background::{Mk48BackgroundContext, Mk48OverlayContext};
use crate::interpolated::Interpolated;
use crate::interpolated_contact::InterpolatedContact;
use crate::settings::Mk48Settings;
use crate::sprite::SortableSprite;
use crate::state::Mk48State;
use crate::ui::{DeathReasonModel, UiEvent, UiProps, UiState, UiStatus};
use client_util::context::Context;
use client_util::fps_monitor::FpsMonitor;
use client_util::game_client::GameClient;
use client_util::joystick::Joystick;
use client_util::keyboard::Key;
use client_util::mouse::{MouseButton, MouseEvent};
use client_util::rate_limiter::RateLimiter;
use client_util::renderer::background::{BackgroundContext, BackgroundLayer};
use client_util::renderer::graphic::GraphicLayer;
use client_util::renderer::particle::{Particle, ParticleLayer};
use client_util::renderer::renderer::Layer;
use client_util::renderer::renderer::Renderer;
use client_util::renderer::sprite::SpriteLayer;
use client_util::renderer::text::TextLayer;
use client_util::rgb::{gray, rgb, rgba};
use common::altitude::Altitude;
use common::angle::Angle;
use common::contact::{Contact, ContactTrait};
use common::entity::{EntityData, EntityId, EntityKind, EntitySubKind, EntityType};
use common::guidance::Guidance;
use common::protocol::{Command, Control, Fire, Hint, Pay, Spawn, Update, Upgrade};
use common::ticks::Ticks;
use common::transform::Transform;
use common::util::score_to_level;
use common::velocity::Velocity;
use common::world::strict_area_border;
use common_util::range::{gen_radius, lerp, map_ranges};
use core_protocol::id::{GameId, TeamId};
use core_protocol::name::PlayerAlias;
use glam::{Mat2, UVec2, Vec2, Vec4, Vec4Swizzles};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::f32::consts::PI;

pub struct Mk48Game {
    /// Can't reverse on first control when you spawn.
    pub first_control: bool,
    /// Holding mouse down, for the purpose of deciding whether to start reversing.
    pub holding: bool,
    /// Currently in mouse control reverse mode.
    pub reversing: bool,
    /// Camera on death.
    pub saved_camera: Option<(Vec2, f32)>,
    /// Override respawning with regular spawning.
    respawn_overridden: bool,
    /// Interpolate altitude for smooth animation of visual range and restriction.
    pub interpolated_altitude: Interpolated,
    /// In meters.
    pub interpolated_zoom: f32,
    /// 1 = normal.
    pub zoom_input: f32,
    /// Last control, for diffing.
    pub last_control: Option<Control>,
    /// Rate limit control websocket messages.
    pub control_rate_limiter: RateLimiter,
    /// Rate limit ui props messages.
    pub ui_props_rate_limiter: RateLimiter,
    /// Playing the alarm fast sound too often is annoying.
    pub alarm_fast_rate_limiter: RateLimiter,
    /// Peek update sound rate limiter (prevent backlog of sounds from nuking ears).
    /// It is reset to 0 every frame, and incremented in every peeked update. When it reaches
    /// a certain threshold, remaining audio and animations are dropped.
    pub peek_update_sound_counter: u8,
    /// If a given index is present and non-zero, should avoid firing weapon (was fired recently,
    /// and is probably consumed).
    pub fire_rate_limiter: FireRateLimiter,
    /// FPS counter
    pub fps_counter: FpsMonitor,
}

/// Order of fields is order of rendering.
#[derive(Layer)]
pub struct RendererLayer {
    background: BackgroundLayer<Mk48BackgroundContext>,
    pub sea_level_particles: ParticleLayer,
    sprites: SpriteLayer,
    pub airborne_particles: ParticleLayer,
    airborne_graphics: GraphicLayer,
    overlay: BackgroundLayer<Mk48OverlayContext>,
    graphics: GraphicLayer,
    text: TextLayer,
}

pub fn wind() -> Vec2 {
    Vec2::new(7.0, 1.5)
}

// Back 75 degrees is reverse angle.
const REVERSE_ANGLE: f32 = PI * 3.0 / 8.0;

impl Mk48Game {
    // Don't reverse early on, when the player doesn't have a great idea of their orientation.
    fn can_reverse(&self, player_contact: &Contact) -> bool {
        self.has_reverse(player_contact)
            && (!self.first_control || player_contact.guidance().velocity_target < Velocity::ZERO)
    }

    // Level 1 ships can't reverse with mouse controls.
    fn has_reverse(&self, player_contact: &Contact) -> bool {
        player_contact.entity_type().unwrap().data().level > 1
    }
}

impl GameClient for Mk48Game {
    const GAME_ID: GameId = GameId::Mk48;

    type Audio = Audio;
    type GameRequest = Command;
    type RendererLayer = RendererLayer;
    type GameState = Mk48State;
    type UiEvent = UiEvent;
    type UiState = UiState;
    type UiProps = UiProps;
    type GameUpdate = Update;
    type GameSettings = Mk48Settings;

    fn new() -> Self {
        unsafe {
            // SAFETY: First thing to run, happens before any entity data loading.
            EntityType::init();
        }

        Self {
            first_control: false,
            holding: false,
            reversing: false,
            interpolated_altitude: Interpolated::new(0.2),
            interpolated_zoom: Self::DEFAULT_ZOOM_INPUT * Self::MENU_VISUAL_RANGE,
            zoom_input: Self::DEFAULT_ZOOM_INPUT,
            saved_camera: None,
            respawn_overridden: false,
            last_control: None,
            control_rate_limiter: RateLimiter::new(0.1),
            ui_props_rate_limiter: RateLimiter::new(0.25),
            alarm_fast_rate_limiter: RateLimiter::new(10.0),
            peek_update_sound_counter: 0,
            fire_rate_limiter: FireRateLimiter::new(),
            fps_counter: FpsMonitor::new(1.0),
        }
    }

    fn init_settings(&mut self, renderer: &mut Renderer) -> Self::GameSettings {
        let animations = !renderer.fragment_uses_mediump();
        let wave_quality = renderer.fragment_has_highp() as u8;

        Mk48Settings {
            animations,
            wave_quality,
            ..Default::default()
        }
    }

    fn init_layer(
        &mut self,
        renderer: &mut Renderer,
        context: &mut Context<Self>,
    ) -> Self::RendererLayer {
        renderer.set_background_color(Vec4::new(0.0, 0.20784314, 0.45490196, 1.0));

        let sprite_sheet = serde_json::from_str(include_str!("./sprites_webgl.json")).unwrap();
        let sprite_texture =
            renderer.load_texture("/sprites_webgl.png", UVec2::new(2048, 2048), None, false);

        let background_context = Mk48BackgroundContext::new(
            renderer,
            context.settings.animations,
            context.settings.wave_quality,
        );

        let overlay_context = Mk48OverlayContext::default();

        if context.settings.animations {
            // Animations on, we can afford more state changes.
            self.ui_props_rate_limiter.set_period(0.1);
        }

        RendererLayer {
            background: BackgroundLayer::new(renderer, background_context),
            sea_level_particles: ParticleLayer::new(renderer, Vec2::ZERO),
            sprites: SpriteLayer::new(renderer, sprite_texture, sprite_sheet),
            airborne_particles: ParticleLayer::new(renderer, wind()),
            airborne_graphics: GraphicLayer::new(renderer),
            overlay: BackgroundLayer::new(renderer, overlay_context),
            graphics: GraphicLayer::new(renderer),
            text: TextLayer::new(renderer),
        }
    }

    /// This violates the normal "peek" contract by doing the work of apply, when it comes to contacts.
    fn peek_game(
        &mut self,
        update: &Update,
        context: &mut Context<Self>,
        renderer: &Renderer,
        _layer: &mut Self::RendererLayer,
    ) {
        self.peek_update_sound_counter = self.peek_update_sound_counter.saturating_add(1);
        // Only play sounds for 10 peeked updates between frames.
        let play_sounds = self.peek_update_sound_counter < 10;

        let updated: HashMap<EntityId, &Contact> =
            update.contacts.iter().map(|c| (c.id(), c)).collect();

        for (id, &contact) in updated.iter() {
            if let Some(InterpolatedContact { model, .. }) = context.state.game.contacts.get(id) {
                if Some(*id) == context.state.game.entity_id {
                    let recent_damage = contact.damage().saturating_sub(model.damage());
                    if recent_damage > Ticks::ZERO {
                        if play_sounds {
                            context.audio.play(Audio::Damage);
                        }

                        // Considered "intense" 250% of the damage would have been fatal.
                        if play_sounds
                            && recent_damage * 2.5
                                >= model.data().max_health().saturating_sub(model.damage())
                        {
                            Self::play_music(Audio::Intense, &context.audio);
                        }
                    }
                }

                // Mutable borrow after immutable borrows.
                let network_contact = context.state.game.contacts.get_mut(id).unwrap();
                network_contact.model = contact.clone();

                // Compensate for the fact that the data is a little old (second parameter is rough
                // estimate of latency)
                network_contact.model.simulate(0.1);
            } else {
                if play_sounds {
                    self.play_new_contact_audio(
                        contact,
                        renderer.camera_center(),
                        &*context,
                        &context.audio,
                    );
                }
                if contact.player_id() == context.state.core.player_id && contact.is_boat() {
                    context.state.game.entity_id = Some(contact.id());
                    self.first_control = true; // Just spawned so reset this.
                    self.interpolated_altitude.reset();
                }
                context
                    .state
                    .game
                    .contacts
                    .insert(contact.id(), InterpolatedContact::new(contact.clone()));
            }
        }

        // Contacts absent in the update are currently considered lost.
        // Borrow entity_id early to avoid use of self in closure.
        let entity_id = &mut context.state.game.entity_id;
        for contact in context
            .state
            .game
            .contacts
            .drain_filter(|id, InterpolatedContact { idle, view, .. }| {
                if updated.contains_key(id) {
                    *idle = Ticks::ZERO;
                    false
                } else {
                    *idle = idle.saturating_add(Ticks::ONE);
                    if *idle
                        <= view
                            .entity_type()
                            .map(|t| *t.data().kind.keep_alive().end())
                            .unwrap_or(EntityKind::MAX_KEEP_ALIVE)
                    {
                        // Still in keep alive period.
                        return false;
                    }
                    if Some(*id) == *entity_id {
                        *entity_id = None;
                    }
                    true
                }
            })
            .map(|(_, InterpolatedContact { view, .. })| view)
            .collect::<Vec<_>>()
        {
            if play_sounds {
                let time_seconds = context.client.update_seconds;
                self.play_lost_contact_audio_and_animations(
                    renderer.camera_center(),
                    &contact,
                    &context.audio,
                    &mut context.state.game.animations,
                    time_seconds,
                );
            }
        }

        let player_position = renderer.camera_center();
        let player_altitude = context
            .state
            .game
            .player_contact()
            .map(|c| c.altitude())
            .unwrap_or(Altitude::ZERO);

        let mut aircraft_volume: f32 = 0.0;
        let mut jet_volume: f32 = 0.0;
        let mut need_to_dodge: f32 = 0.0;

        for (_, InterpolatedContact { view: contact, .. }) in context.state.game.contacts.iter() {
            if let Some(entity_type) = contact.entity_type() {
                let data: &'static EntityData = entity_type.data();
                let position_diff = contact.transform().position - player_position;
                let direction = Angle::from(position_diff);
                let distance = position_diff.length();
                let inbound =
                    (contact.transform().direction - direction + Angle::PI).abs() < Angle::PI_2;

                let friendly = context.state.core.is_friendly(contact.player_id());
                let volume = Self::volume_at(distance);

                if data.kind == EntityKind::Aircraft {
                    if matches!(entity_type, EntityType::SuperEtendard) {
                        jet_volume += volume;
                    } else {
                        aircraft_volume += volume;
                    }
                }

                if context.state.game.entity_id.is_some() && distance < 250.0 {
                    let distance_scale = 1000.0 / (500.0 + distance);
                    match data.kind {
                        EntityKind::Boat => {
                            if !friendly
                                && inbound
                                && data.sub_kind == EntitySubKind::Ram
                                && !player_altitude.is_submerged()
                            {
                                need_to_dodge += 2.0 * distance_scale;
                            }
                        }
                        EntityKind::Weapon => match data.sub_kind {
                            EntitySubKind::Torpedo => {
                                if inbound && !friendly {
                                    need_to_dodge += distance_scale;
                                }
                            }
                            EntitySubKind::DepthCharge | EntitySubKind::Mine => {
                                if !friendly {
                                    need_to_dodge += distance_scale;
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }

        if aircraft_volume > 0.01 {
            context
                .audio
                .play_with_volume(Audio::Aircraft, (aircraft_volume + 1.0).ln());
        }

        if jet_volume > 0.01 {
            context
                .audio
                .play_with_volume(Audio::Jet, (jet_volume + 1.0).ln());
        }

        if need_to_dodge >= 3.0 {
            Self::play_music(Audio::Dodge, &context.audio);
        }

        let score_delta = update.score.saturating_sub(context.state.game.score);
        if score_delta >= 10
            && (score_delta >= 200 || score_delta as f32 / context.state.game.score as f32 > 0.5)
        {
            Self::play_music(Audio::Achievement, &context.audio);
        }
    }

    fn peek_mouse(
        &mut self,
        event: &MouseEvent,
        _context: &mut Context<Self>,
        _renderer: &Renderer,
    ) {
        if let MouseEvent::Wheel(delta) = event {
            self.zoom(*delta);
        }
    }

    fn tick(
        &mut self,
        elapsed_seconds: f32,
        context: &mut Context<Self>,
        renderer: &mut Renderer,
        layer: &mut Self::RendererLayer,
    ) {
        // Allow more sounds to be played in peek.
        self.peek_update_sound_counter = 0;

        // The distance from player's boat to the closest visible member of each team, for the purpose of sorting and
        // filtering.
        let mut team_proximity: HashMap<TeamId, f32> = HashMap::new();

        // Temporary (will be recalculated after moving ships).
        self.update_camera(
            context.state.game.player_contact(),
            elapsed_seconds,
            layer.background.context.frame_cache_enabled(),
        );
        let (camera, _) = self.camera(context.state.game.player_contact(), renderer.aspect_ratio());

        // Cannot borrow entire context, do this instead.
        let connection_lost = context.connection_lost();

        // Update audio volume.
        if Self::maybe_contact_mut(
            &mut context.state.game.contacts,
            context.state.game.entity_id,
        )
        .is_some()
            || context.state.game.death_reason.is_some()
        {
            context.audio.set_muted_by_game(false);
            if !context.audio.is_playing(Audio::Ocean) {
                context.audio.play_looping(Audio::Ocean);
            }
        } else {
            context.audio.set_muted_by_game(true);
            self.last_control = None;
        }

        let debug_latency_entity_id = if false {
            context.state.game.entity_id
        } else {
            None
        };
        // A subset of game logic.
        for interp in &mut context.state.game.contacts.values_mut() {
            if interp
                .model
                .entity_type()
                .map(|e| e.data().kind == EntityKind::Boat)
                .unwrap_or(false)
            {
                // Update team_proximity.
                if let Some(player_id) = interp.model.player_id() {
                    if let Some(player) = context.state.core.only_players().get(&player_id) {
                        if let Some(team_id) = player.team_id {
                            let mut distance =
                                camera.distance_squared(interp.model.transform().position);

                            if let Some(team) = context.state.core.teams.get(&team_id) {
                                if team.closed {
                                    distance *= 10.0;
                                }
                            }

                            team_proximity
                                .entry(team_id)
                                .and_modify(|dist| *dist = dist.min(distance))
                                .or_insert(distance);
                        }
                    }
                }
            }

            interp.update_error_bound(elapsed_seconds, debug_latency_entity_id);
            interp.generate_particles(layer);
            interp.interpolate(elapsed_seconds, context.state.game.entity_id);
        }

        // May have changed due to the above.
        let (camera, zoom) =
            self.camera(context.state.game.player_contact(), renderer.aspect_ratio());

        // Set camera before update layers so they don't get last frame's camera.
        // TODO decouple update and render.
        renderer.set_camera(camera, zoom);

        let (visual_range, visual_restriction, area) =
            if let Some(c) = context.state.game.player_interpolated_contact() {
                // Use model as input to interpolation (can't interpolate twice).
                let altitude = c.model.altitude().to_norm();
                let t = context.client.update_seconds;
                let altitude = self.interpolated_altitude.update(altitude, t);

                // Use view for entity type.
                let entity_type = c.view.entity_type().unwrap();

                let visual_range = entity_type.data().sensors.visual.range
                    * map_ranges(altitude, -1.0..0.0, 0.4..0.8, true);
                let visual_restriction = map_ranges(altitude, 0.0..-1.0, 0.0..0.8, true);
                let area = strict_area_border(entity_type);
                (visual_range, visual_restriction, area)
            } else {
                (500.0, 0.0, None)
            };

        // Prepare to sort sprites.
        let mut sortable_sprites = Vec::with_capacity(context.state.game.contacts.len() * 5);

        // Update background and add vegetation sprites.
        let terrain_reset = context.state.game.take_terrain_reset();
        sortable_sprites.extend(layer.background.context.update(
            camera,
            zoom,
            &mut context.state.game.terrain,
            terrain_reset,
            &*renderer,
        ));

        layer.overlay.context.update(
            visual_range,
            visual_restriction,
            context.state.game.world_radius,
            area,
        );

        let mut anti_aircraft_volume = 0.0;

        // Update animations.
        let mut i = 0;
        while i < context.state.game.animations.len() {
            let animation = &mut context.state.game.animations[i];

            let len = layer.sprites.animation_length(animation.name);

            if animation.frame(context.client.update_seconds) >= len {
                context.state.game.animations.swap_remove(i);
            } else {
                sortable_sprites.push(SortableSprite::new_animation(
                    animation,
                    context.client.update_seconds,
                ));
                i += 1;
            }
        }

        // Update trails.
        context
            .state
            .game
            .trails
            .set_time(context.client.update_seconds);

        for InterpolatedContact { view: contact, .. } in context.state.game.contacts.values() {
            let friendly = context.state.core.is_friendly(contact.player_id());

            let color = if friendly {
                rgb(58, 255, 140)
            } else if contact.is_boat() {
                gray(255)
            } else {
                rgb(231, 76, 60)
            };

            if let Some(entity_type) = contact.entity_type() {
                let altitude = contact.altitude().to_norm();
                // Only boats have non linear altitude to alpha (because they have more depth).
                // TODO give entities depth dimension.
                let alpha = if !contact.is_boat() || contact.altitude().is_submerged() {
                    let max_alpha = if contact.is_boat() { 0.7 } else { 1.0 };
                    map_ranges(altitude, -1.0..0.0, 0.2..max_alpha, true)
                } else {
                    1.0
                };
                let entity_id = contact.id();
                let data: &'static EntityData = entity_type.data();

                {
                    let mut transform = *contact.transform();
                    let settings = &context.settings;

                    // Subtle wave animation on collectibles.
                    // No waves if animations or waves are off.
                    if settings.animations
                        && settings.wave_quality > 0
                        && data.kind == EntityKind::Collectible
                    {
                        let t = context.client.update_seconds;

                        // Moves waves with the wind.
                        let mut input = (transform.position + wind() * t) * 0.1;

                        // Offset waves from regular grid.
                        input.x += input.y * 0.3;

                        // Don't apply waves when collectibles are moving (aka attracted to boat).
                        let f = map_ranges(transform.velocity.to_mps(), 0.0..4.0, 1.0..0.0, true);

                        // Waves modify rendered position and direction.
                        transform.position += Vec2::new(input.x.sin(), input.y.sin()) * (f * 0.9);
                        transform.direction += Angle::from_radians(
                            input.dot(Vec2::new(0.7, 1.3)).sin() * (f * (PI / 10.0)),
                        );
                    }

                    sortable_sprites.push(SortableSprite::new_entity(
                        entity_id,
                        entity_type,
                        transform,
                        altitude,
                        alpha,
                    ));
                }

                if contact.is_boat()
                    && !contact.altitude().is_submerged()
                    && data.anti_aircraft > 0.0
                {
                    anti_aircraft_volume += Self::simulate_anti_aircraft(
                        contact,
                        &context.state.game.contacts,
                        &context.state.core,
                        renderer.camera_center(),
                        &mut layer.airborne_particles,
                    );
                }

                if contact.is_boat() {
                    for i in 0..data.armaments.len() {
                        let armament = &data.armaments[i];
                        if armament.hidden
                            || armament.vertical
                            || !(armament.external || (friendly && !context.ui.cinematic))
                        {
                            continue;
                        }
                        let armament_type = armament.entity_type;
                        sortable_sprites.push(SortableSprite::new_child_entity(
                            entity_id,
                            entity_type,
                            armament_type,
                            *contact.transform() + data.armament_transform(contact.turrets(), i),
                            altitude + 0.02,
                            alpha
                                * if contact.reloads().get(i).map(|r| *r).unwrap_or(false) {
                                    1.0
                                } else {
                                    0.5
                                },
                        ));
                    }
                }
                for (i, turret) in data.turrets.iter().enumerate() {
                    if let Some(turret_type) = turret.entity_type {
                        let pos = turret.position();
                        sortable_sprites.push(SortableSprite::new_child_entity(
                            entity_id,
                            entity_type,
                            turret_type,
                            *contact.transform()
                                + Transform {
                                    position: pos,
                                    direction: contact.turrets()[i],
                                    velocity: Velocity::ZERO,
                                }
                                + Transform {
                                    position: turret_type.data().offset(),
                                    direction: Angle::ZERO,
                                    velocity: Velocity::ZERO,
                                },
                            altitude + 0.02 - (pos.x.abs() + pos.y.abs()) * 0.0001,
                            alpha,
                        ));
                    }
                }

                // GUI overlays.
                let overlay_vertical_position = data.radius * 1.2;

                match data.kind {
                    EntityKind::Boat => {
                        // Is this player's own boat?
                        if context.state.core.player_id.is_some()
                            && contact.player_id() == context.state.core.player_id
                            && !context.ui.cinematic
                        {
                            // Radii
                            let hud_color = rgba(255, 255, 255, 255 / 3);
                            let reverse_color = rgba(255, 75, 75, 120);
                            let hud_thickness = 0.0025 * zoom;

                            // Throttle rings.
                            // 1. Inner
                            layer.graphics.add_circle(
                                contact.transform().position,
                                data.radii().start,
                                hud_thickness,
                                hud_color,
                            );
                            // 2. Outer
                            layer.graphics.add_circle(
                                contact.transform().position,
                                data.radii().end,
                                hud_thickness,
                                hud_color,
                            );
                            // 3. Actual speed
                            layer.graphics.add_circle(
                                contact.transform().position,
                                map_ranges(
                                    contact.transform().velocity.abs().to_mps(),
                                    0.0..data.speed.to_mps(),
                                    data.radii(),
                                    false,
                                ),
                                hud_thickness,
                                if contact.transform().velocity < Velocity::ZERO {
                                    reverse_color
                                } else {
                                    hud_color
                                },
                            );
                            // 4. Target speed
                            {
                                let velocity_target = contact
                                    .guidance()
                                    .velocity_target
                                    .max(data.speed * Velocity::MAX_REVERSE_SCALE);
                                layer.graphics.add_circle(
                                    contact.transform().position,
                                    map_ranges(
                                        velocity_target.abs().to_mps(),
                                        0.0..data.speed.to_mps(),
                                        data.radii(),
                                        true,
                                    ),
                                    hud_thickness,
                                    if velocity_target < Velocity::ZERO {
                                        reverse_color
                                    } else {
                                        hud_color
                                    },
                                );
                            }

                            // Target bearing line.
                            {
                                let guidance = contact.guidance();
                                let mut direction = guidance.direction_target;
                                let mut color = hud_color;

                                // Is reversing.
                                // Fix ambiguity when loading guidance from server.
                                if guidance.velocity_target < Velocity::ZERO
                                    || guidance.velocity_target == Velocity::ZERO && self.reversing
                                {
                                    // Flip dir if going backwards.
                                    direction += Angle::PI;
                                    // Reversing color.
                                    color = reverse_color;
                                    // TODO BEEP BEEP BEEP
                                }

                                let dir_mat = Mat2::from_angle(direction.to_radians());
                                let radii = data.radii();
                                // Don't overlap inner/outer hud circles.
                                let start = radii.start + hud_thickness * 0.5;
                                let end = radii.end - hud_thickness * 0.5;

                                layer.graphics.add_line(
                                    contact.transform().position + dir_mat * Vec2::new(start, 0.0),
                                    contact.transform().position + dir_mat * Vec2::new(end, 0.0),
                                    hud_thickness,
                                    color,
                                );
                            }

                            // Reverse azimuths.
                            if self.has_reverse(contact) {
                                let mut range = data.radii();
                                // Don't overlap inner hud circle.
                                range.start += hud_thickness * 0.5;
                                range.end = lerp(range.start, range.end, 0.1);

                                let position = contact.transform().position;
                                let direction = contact.transform().direction.to_radians() - PI;
                                let mut color = hud_color;

                                if self.holding || !self.can_reverse(contact) {
                                    if self.reversing {
                                        color = reverse_color
                                    } else {
                                        color.w *= 0.5;
                                    }
                                }
                                let end_color = color.xyz().extend(0.0);

                                for m in [-0.5, 0.5] {
                                    let direction = direction + REVERSE_ANGLE * m;
                                    let dir_mat = Mat2::from_angle(direction);

                                    layer.graphics.add_line_gradient(
                                        position + dir_mat * Vec2::new(range.start, 0.0),
                                        position + dir_mat * Vec2::new(range.end, 0.0),
                                        hud_thickness,
                                        color,
                                        end_color,
                                    );
                                }
                            }

                            // Turret azimuths.
                            // Pre-borrow to not borrow all of context (will be fixed eventually).
                            let ui_armament = context.ui.armament;
                            if let Some((i, mouse_pos)) =
                                context.mouse.world_position.and_then(|mouse_pos| {
                                    self.find_best_armament(contact, false, mouse_pos, ui_armament)
                                        .zip(Some(mouse_pos))
                                })
                            {
                                let armament = &data.armaments[i];
                                if armament.entity_type != EntityType::Depositor {
                                    let transform = *contact.transform();
                                    let direction = contact.transform().direction;
                                    let color = hud_color;

                                    if let Some(turret_index) = armament.turret {
                                        let turret = &data.turrets[turret_index];
                                        let turret_radius = turret
                                            .entity_type
                                            .map(|e| e.data().radius)
                                            .unwrap_or(0.0);
                                        let transform =
                                            transform + Transform::from_position(turret.position());

                                        let inner: f32 = 0.2 * data.width;
                                        let outer: f32 = 0.325 * data.width;
                                        let arc_thickness: f32 = outer - inner;
                                        let middle: f32 = inner + arc_thickness * 0.5;

                                        // Aim line is only helpful on small turrets.
                                        if turret_radius < inner {
                                            let turret_direction = (direction
                                                + contact.turrets()[turret_index])
                                                .to_radians();
                                            let dir_mat = Mat2::from_angle(turret_direction);
                                            let line_thickness = hud_thickness * 2.0;

                                            layer.graphics.add_line(
                                                transform.position
                                                    + dir_mat * Vec2::new(inner, 0.0),
                                                transform.position
                                                    + dir_mat * Vec2::new(outer, 0.0),
                                                line_thickness,
                                                color,
                                            );
                                        }

                                        let left_back = (direction + turret.angle
                                            - turret.azimuth_bl
                                            + Angle::PI)
                                            .to_radians();
                                        let left_front =
                                            (direction + turret.angle + turret.azimuth_fl)
                                                .to_radians();
                                        let right_back = (direction
                                            + turret.angle
                                            + turret.azimuth_br
                                            + Angle::PI)
                                            .to_radians();
                                        let right_front = (direction + turret.angle
                                            - turret.azimuth_fr)
                                            .to_radians();

                                        if turret.azimuth_fr + turret.azimuth_br < Angle::PI {
                                            layer.graphics.add_arc(
                                                transform.position,
                                                middle,
                                                right_back..if right_front > right_back {
                                                    right_front
                                                } else {
                                                    right_front + 2.0 * std::f32::consts::PI
                                                },
                                                arc_thickness,
                                                color,
                                            );
                                        }
                                        if turret.azimuth_fl + turret.azimuth_bl < Angle::PI {
                                            layer.graphics.add_arc(
                                                transform.position,
                                                middle,
                                                left_front..if left_back > left_front {
                                                    left_back
                                                } else {
                                                    left_back + 2.0 * PI
                                                },
                                                arc_thickness,
                                                color,
                                            );
                                        }
                                    } else {
                                        let armament_data = armament.entity_type.data();

                                        let transform = transform
                                            + Transform::from_position(armament.position());
                                        let color = hud_color.xyz().extend(0.45);

                                        let is_vertical = armament.vertical;
                                        let is_stationary = matches!(
                                            armament_data.sub_kind,
                                            EntitySubKind::DepthCharge | EntitySubKind::Mine
                                        );

                                        if !is_stationary {
                                            let direction = if is_vertical
                                                || armament_data.sub_kind == EntitySubKind::Heli
                                            {
                                                (mouse_pos - transform.position).into()
                                            } else {
                                                direction + armament.angle
                                            };

                                            let angle_radians = direction.to_radians();
                                            let dir_mat = Mat2::from_angle(angle_radians);

                                            let scale = data.width * 0.15;
                                            let start: f32 = (0.65 * armament_data.length)
                                                .max(data.width * 0.2)
                                                + scale * 0.5;
                                            let center = transform.position
                                                + dir_mat * Vec2::new(start, 0.0);

                                            // Widen triangle for aircraft.
                                            let scale_x =
                                                if armament_data.kind == EntityKind::Aircraft {
                                                    scale
                                                } else {
                                                    scale * (2.0 / 3.0)
                                                };
                                            let scale = Vec2::new(scale_x, scale);

                                            layer.graphics.add_triangle(
                                                center,
                                                scale,
                                                angle_radians - PI * 0.5,
                                                color,
                                            );
                                        } else {
                                            let inner: f32 = 0.11 * data.width;
                                            layer.graphics.add_circle(
                                                transform.position,
                                                inner,
                                                inner * 0.55,
                                                color,
                                            )
                                        }

                                        if is_vertical {
                                            let inner: f32 = 0.055 * data.width;
                                            layer.graphics.add_filled_circle(
                                                transform.position,
                                                inner,
                                                color,
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Health bar
                        if contact.damage() > Ticks::ZERO {
                            let length = 0.12 * zoom;
                            let health =
                                1.0 - contact.damage().to_secs() / data.max_health().to_secs();
                            let center = contact.transform().position
                                + Vec2::new(0.0, overlay_vertical_position);
                            let offset_x = |v, x| v + Vec2::new(x, 0.0);

                            let bg_color = rgba(85, 85, 85, 127);
                            let health_color = color.extend(1.0);
                            let thickness = 0.0075 * zoom;

                            // Background of health bar.
                            layer.graphics.add_rounded_line(
                                offset_x(center, length * -0.5),
                                offset_x(center, length * 0.5),
                                thickness,
                                bg_color,
                                true,
                            );

                            // Health indicator.
                            layer.graphics.add_rounded_line(
                                offset_x(center, length * -0.5),
                                offset_x(center, length * (health - 0.5)),
                                thickness,
                                health_color,
                                true,
                            );
                        }

                        // Name
                        let text = if let Some(player) = context
                            .state
                            .core
                            .player_or_bot(contact.player_id().unwrap())
                        {
                            if let Some(team) = player
                                .team_id
                                .and_then(|team_id| context.state.core.teams.get(&team_id))
                            {
                                format!("[{}] {}", team.name, player.alias)
                            } else {
                                player.alias.as_str().to_owned()
                            }
                        } else {
                            // This is not meant to happen in production. It is for debugging.
                            format!("{}", contact.player_id().unwrap().0.get())
                        };

                        layer.text.add(
                            text,
                            contact.transform().position
                                + Vec2::new(0.0, overlay_vertical_position + 0.035 * zoom),
                            0.035 * zoom,
                            color.extend(1.0),
                        );
                    }
                    EntityKind::Weapon | EntityKind::Decoy | EntityKind::Aircraft => {
                        let triangle_position = contact.transform().position
                            + Vec2::new(0.0, overlay_vertical_position);
                        layer.graphics.add_triangle(
                            triangle_position + Vec2::new(0.0, 0.01 * zoom),
                            Vec2::splat(0.02 * zoom),
                            180f32.to_radians(),
                            color.extend(1.0),
                        );
                    }
                    _ => {}
                }

                // Add particles.
                let mut rng = thread_rng();
                let direction_vector: Vec2 = contact.transform().direction.into();
                let tangent_vector = direction_vector.perp();
                let speed = contact.transform().velocity.to_mps();

                // Integer amount of particles from fractional per_second
                let amount = {
                    let per_second = data.width * 6.0 + speed * 2.0;
                    let t = context.client.update_seconds;
                    let time_delta = elapsed_seconds;

                    // Essentially a random number based on time.
                    const SUB_STEPS: f32 = 1000.0 * PI;
                    let f = (t as f64 * SUB_STEPS as f64).fract() as f32;

                    // Clamp to 100 just in case floats aren't being very nice.
                    // Yamato makes ~30 particles per frame (60 fps) so it's plenty.
                    ((time_delta * per_second + f).floor() as usize).min(100)
                };

                // Wake/thrust particles and shell trails.
                if contact.transform().velocity != Velocity::ZERO
                    && (data.sub_kind != EntitySubKind::Submarine
                        || contact.transform().velocity > data.cavitation_speed(contact.altitude()))
                {
                    if data.sub_kind == EntitySubKind::Shell {
                        let t = contact.transform();
                        context.state.game.trails.add_trail(
                            entity_id,
                            t.position,
                            t.direction.to_vec() * t.velocity.to_mps(),
                            data.width * 2.0,
                        );
                    } else {
                        let is_airborne = contact.altitude().is_airborne();
                        let layer = if is_airborne {
                            &mut layer.airborne_particles
                        } else {
                            &mut layer.sea_level_particles
                        };

                        let spread = match (data.kind, data.sub_kind) {
                            _ if !is_airborne => 0.16,
                            (EntityKind::Aircraft, _) => 0.08,
                            (
                                EntityKind::Weapon,
                                EntitySubKind::Rocket | EntitySubKind::RocketTorpedo,
                            ) => 0.08,
                            _ => 0.04,
                        };

                        let start = contact.transform().position;
                        let end = start + direction_vector * speed * elapsed_seconds;

                        let factor = 1.0 / amount as f32;
                        for i in 0..amount {
                            let r = rng.gen::<f32>() - 0.5;

                            let position = start.lerp(end, i as f32 * factor)
                                - direction_vector * (data.length * 0.485)
                                + tangent_vector * (data.width * r * 0.25);

                            let velocity = direction_vector * (speed * 0.75)
                                + tangent_vector * (speed * r * spread);

                            layer.add(Particle {
                                position,
                                velocity,
                                radius: 1.0,
                                color: 1.0,
                                smoothness: 1.0,
                            });
                        }
                    }
                }

                // Exhaust particles
                if !contact.altitude().is_submerged() {
                    for exhaust in data.exhausts.iter() {
                        for _ in 0..amount * 2 {
                            layer.airborne_particles.add(Particle {
                                position: contact.transform().position
                                    + direction_vector * exhaust.position_forward
                                    + tangent_vector * exhaust.position_side
                                    + gen_radius(&mut rng, 1.5),
                                velocity: gen_radius(&mut rng, 6.0),
                                radius: 1.0,
                                color: if entity_type == EntityType::OilPlatform {
                                    -1.0
                                } else {
                                    0.4
                                },
                                smoothness: 1.0,
                            });
                        }
                    }
                }
            } else {
                layer.sprites.add(
                    "contact",
                    None,
                    contact.transform().position,
                    Vec2::splat(10.0),
                    contact.transform().direction,
                    1.0,
                );
            }
        }

        context
            .state
            .game
            .trails
            .update(&mut layer.airborne_graphics);

        // Play anti-aircraft sfx.
        if anti_aircraft_volume > 0.0 && !context.audio.is_playing(Audio::Aa) {
            context
                .audio
                .play_with_volume(Audio::Aa, anti_aircraft_volume.min(0.5));
        }

        // Sort sprites by altitude.
        sortable_sprites.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        for s in sortable_sprites {
            layer.sprites.add(
                s.sprite,
                s.frame,
                s.transform.position,
                s.dimensions,
                s.transform.direction,
                s.alpha,
            );
        }

        // After the above line, mouse world position state may be out-of-date. Recalculate it here.
        let aim_target = context
            .mouse
            .view_position
            .map(|p| renderer.to_world_position(p));

        // Send command later, when lifetimes allow.
        let mut control: Option<Command> = None;

        let status = if let Some(player_contact) = Self::maybe_contact_mut(
            &mut context.state.game.contacts,
            context.state.game.entity_id,
        ) {
            let mut guidance = None;

            {
                let player_contact = &player_contact.view;
                let max_speed = player_contact.data().speed.to_mps();

                let joystick = Joystick::try_from_keyboard_state(
                    context.client.update_seconds,
                    &context.keyboard,
                );
                let stop = joystick.as_ref().map(|j| j.stop).unwrap_or(false);

                if let Some(joystick) = joystick {
                    guidance = Some(Guidance {
                        direction_target: player_contact.transform().direction
                            + Angle::from_radians(0.5 * joystick.position.x),
                        velocity_target: if joystick.stop {
                            Velocity::ZERO
                        } else if joystick.position.y.abs() > 0.05 {
                            player_contact.transform().velocity
                                + Velocity::from_mps(0.25 * max_speed * joystick.position.y)
                        } else {
                            player_contact.guidance().velocity_target
                        },
                    });
                    self.first_control = false; // First control was joystick.
                }

                if context.mouse.is_down(MouseButton::Right)
                    || context
                        .mouse
                        .is_down_not_click(MouseButton::Left, context.client.update_seconds)
                {
                    let current_dir = player_contact.transform().direction;
                    let mut direction_target = Angle::from(
                        aim_target.unwrap_or_default() - player_contact.transform().position,
                    );

                    // Only do when start holding.
                    if !self.holding {
                        if self.can_reverse(player_contact) {
                            // Starting movement behind ship turns on reverse.
                            let delta = direction_target - current_dir;
                            self.reversing = delta
                                != delta.clamp_magnitude(Angle::from_radians(
                                    (PI * 2.0 - REVERSE_ANGLE) / 2.0,
                                ));
                        }
                        self.first_control = false; // First control was mouse.
                        self.holding = true;
                    }

                    // If reversing flip angle.
                    if self.reversing {
                        direction_target += Angle::PI
                    }

                    if stop {
                        // Limit turning while "stopped"
                        direction_target = current_dir
                            + (direction_target - player_contact.transform().direction)
                                .clamp_magnitude(Angle::from_radians(0.5));
                    }

                    guidance = Some(Guidance {
                        direction_target,
                        velocity_target: if stop {
                            Velocity::ZERO
                        } else {
                            let mut velocity = Velocity::from_mps(map_ranges(
                                aim_target
                                    .unwrap_or_default()
                                    .distance(player_contact.transform().position),
                                player_contact.data().radii(),
                                0.0..max_speed,
                                true,
                            ));
                            if self.reversing {
                                velocity = -velocity;
                            }
                            velocity
                        },
                    });
                } else {
                    self.holding = false;
                    self.reversing = false;
                }
            }

            if let Some(guidance) = guidance.as_ref() {
                player_contact.model.predict_guidance(guidance);
                player_contact.view.predict_guidance(guidance);
            }

            // Re-borrow as immutable.
            let player_contact = context.state.game.player_contact().unwrap();

            let status = UiStatus::Playing {
                entity_type: player_contact.entity_type().unwrap(),
                position: player_contact.transform().position.into(),
                direction: player_contact.transform().direction,
                velocity: player_contact.transform().velocity,
                altitude: player_contact.altitude(),
                armament_consumption: Some(player_contact.reloads().iter().map(|b| *b).collect()),
            };

            if self.control_rate_limiter.update_ready(elapsed_seconds) {
                let left_click = context.mouse.take_click(MouseButton::Left);

                // Get hint before borrow of player_contact().
                let hint = Some(Hint {
                    aspect: renderer.aspect_ratio(),
                });

                let current_control = Control {
                    guidance: Some(*player_contact.guidance()), // TODO don't send if hasn't changed.
                    submerge: if player_contact.data().sub_kind == EntitySubKind::Submarine {
                        context.ui.altitude_target != Altitude::ZERO
                    } else {
                        false
                    },
                    aim_target,
                    active: context.ui.active,
                    pay: context.keyboard.is_down(Key::C).then_some(Pay),
                    fire: if left_click
                        || context
                            .keyboard
                            .state(Key::Space)
                            .combined(context.keyboard.state(Key::E))
                            .is_down()
                    {
                        self.find_best_armament(
                            player_contact,
                            true,
                            aim_target.unwrap_or_default(),
                            context.ui.armament,
                        )
                        .map(|i| {
                            self.fire_rate_limiter.fired(i as u8);

                            Fire {
                                armament_index: i as u8,
                            }
                        })
                    } else {
                        None
                    },
                    hint,
                };

                // Some things are not idempotent.
                fn is_significant(control: &Control) -> bool {
                    control.fire.is_some() || control.pay.is_some()
                }

                if Some(&current_control) != self.last_control.as_ref()
                    || is_significant(&current_control)
                    || self
                        .last_control
                        .as_ref()
                        .map(is_significant)
                        .unwrap_or(false)
                {
                    self.last_control = Some(current_control.clone());
                    control = Some(Command::Control(current_control));
                }
            }

            // Playing, so reset respawn override for next time.
            self.respawn_overridden = false;

            status
        } else if connection_lost {
            UiStatus::Offline
        } else if let Some(death_reason) = context
            .state
            .game
            .death_reason
            .as_ref()
            .filter(|_| !self.respawn_overridden)
            .and_then(|reason| DeathReasonModel::from_death_reason(reason).ok())
        {
            UiStatus::Respawning {
                death_reason,
                respawn_level: score_to_level(context.state.game.score),
            }
        } else {
            UiStatus::Spawning
        };

        if let Some(control) = control {
            context.send_to_game(control);
        }

        self.fps_counter.update(elapsed_seconds);
        self.fire_rate_limiter.update(elapsed_seconds);

        if self.ui_props_rate_limiter.update_ready(elapsed_seconds) {
            self.update_ui_props(context, status, &team_proximity);
        }
    }

    fn peek_ui(
        &mut self,
        event: &UiEvent,
        context: &mut Context<Self>,
        _layer: &mut Self::RendererLayer,
    ) {
        match event {
            UiEvent::Spawn { alias, entity_type } => {
                context.send_set_alias(PlayerAlias::new_unsanitized(alias));
                context.send_to_game(Command::Spawn(Spawn {
                    entity_type: *entity_type,
                }))
            }
            UiEvent::Upgrade(entity_type) => {
                context.audio.play(Audio::Upgrade);
                context.send_to_game(Command::Upgrade(Upgrade {
                    entity_type: *entity_type,
                }))
            }
            UiEvent::Active(active) => {
                if let Some(contact) = context.state.game.player_contact() {
                    if *active && contact.data().sensors.sonar.range >= 0.0 {
                        context.audio.play(Audio::Sonar1);
                    }
                }
            }
            UiEvent::AltitudeTarget(altitude_norm) => {
                let altitude = Altitude::from_norm(*altitude_norm);
                if let Some(contact) = context.state.game.player_contact() {
                    if contact.data().sub_kind == EntitySubKind::Submarine {
                        if !context.ui.altitude_target.is_submerged() && altitude.is_submerged() {
                            context.audio.play(Audio::Dive);
                        } else if context.ui.altitude_target.is_submerged()
                            && !altitude.is_submerged()
                        {
                            context.audio.play(Audio::Surface);
                        }
                    }
                }
            }
            UiEvent::OverrideRespawn => {
                self.respawn_overridden = true;
            }
            _ => {}
        }
    }
}
