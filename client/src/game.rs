// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::armament::{group_armaments, FireRateLimiter, Group};
use crate::audio::Audio;
use crate::background::{Mk48BackgroundLayer, Mk48OverlayLayer};
use crate::camera::Mk48Camera;
use crate::interpolated::Interpolated;
use crate::interpolated_contact::InterpolatedContact;
use crate::particle::{Mk48Particle, Mk48ParticleLayer};
use crate::settings::{Mk48Settings, ShadowSetting};
use crate::sortable_sprite::SortableSprite;
use crate::sprite::SpriteLayer;
use crate::state::Mk48State;
use crate::trail::TrailLayer;
use crate::ui::{
    InstructionStatus, UiEvent, UiProps, UiState, UiStatus, UiStatusPlaying, UiStatusRespawning,
};
use crate::weather::Weather;
use client_util::context::Context;
use client_util::fps_monitor::FpsMonitor;
use client_util::game_client::GameClient;
use client_util::joystick::Joystick;
use client_util::keyboard::{Key, KeyboardEvent};
use client_util::mouse::{MouseButton, MouseEvent, MouseState};
use client_util::rate_limiter::RateLimiter;
use common::altitude::Altitude;
use common::angle::Angle;
use common::contact::{Contact, ContactTrait};
use common::entity::{EntityData, EntityId, EntityKind, EntitySubKind, EntityType};
use common::guidance::Guidance;
use common::protocol::{Command, Control, Fire, Hint, Pay, Spawn, Update, Upgrade};
use common::ticks::Ticks;
use common::transform::Transform;
use common::velocity::Velocity;
use common::world::strict_area_border;
use common_util::range::{gen_radius, lerp, map_ranges};
use core_protocol::id::{GameId, TeamId};
use glam::{Mat2, UVec2, Vec2, Vec3, Vec4Swizzles};
use rand::{thread_rng, Rng};
use renderer::{gray_a, rgb_array, rgba, DefaultRender, Layer, RenderChain};
use renderer2d::{Camera2d, GraphicLayer, TextLayer};
use renderer3d::ShadowLayer;
use renderer3d::{ShadowParams, ShadowResult};
use std::collections::HashMap;
use std::f32::consts::PI;

pub struct Mk48Game {
    /// Mk48 specific camera.
    pub mk48_camera: Mk48Camera,
    /// Game's camera.
    pub camera: Camera2d,
    /// For rendering.
    pub render_chain: RenderChain<FullLayer>,
    /// Can't reverse on first control when you spawn. Also for UI instructions.
    pub first_control: bool,
    /// For UI instructions.
    pub first_zoom: bool,
    /// Holding mouse down, for the purpose of deciding whether to start reversing.
    pub holding: bool,
    /// Currently in mouse control reverse mode.
    pub reversing: bool,
    /// Override respawning with regular spawning.
    respawn_overridden: bool,
    /// Interpolate altitude for smooth animation of visual range and restriction.
    pub interpolated_altitude: Interpolated,
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
    ui_state: UiState,
}

type FullLayer = ShadowLayer<Mk48Layer>;

/// Order of fields is order of rendering.
#[derive(Layer)]
#[render(&ShadowResult<&Mk48Params>)]
#[render(&ShadowParams)]
pub struct Mk48Layer {
    #[render(&ShadowParams)]
    background: Mk48BackgroundLayer,
    pub sea_level_particles: Mk48ParticleLayer<false>,
    // TODO sprite shadows. #[render(&ShadowParams)]
    sprites: SpriteLayer,
    pub airborne_particles: Mk48ParticleLayer<true>,
    trails: TrailLayer,
    overlay: Mk48OverlayLayer,
    graphics: GraphicLayer,
    text: TextLayer,
}

pub struct Mk48Params {
    pub camera: Camera2d,
    pub weather: Weather,
}

impl std::ops::Deref for Mk48Params {
    type Target = Camera2d;
    fn deref(&self) -> &Self::Target {
        &self.camera
    }
}

/// Back 75 degrees is reverse angle.
const REVERSE_ANGLE: f32 = PI * 3.0 / 8.0;
pub const SURFACE_KEY: Key = Key::R;
pub const ACTIVE_KEY: Key = Key::Z;

impl Mk48Game {
    // Don't reverse early on, when the player doesn't have a great idea of their orientation.
    fn can_reverse(first_control: bool, player_contact: &Contact) -> bool {
        Self::has_reverse(player_contact)
            && (!first_control || player_contact.guidance().velocity_target < Velocity::ZERO)
    }

    // Level 1 ships can't reverse with mouse controls.
    fn has_reverse(player_contact: &Contact) -> bool {
        player_contact.entity_type().unwrap().data().level > 1
    }

    // Right button down or left button down and time has passed.
    fn is_holding_control(mouse: &MouseState, time: f32) -> bool {
        mouse.is_down(MouseButton::Right) || mouse.is_down_not_click(MouseButton::Left, time)
    }

    fn create_render_chain(context: &Context<Self>) -> Result<RenderChain<FullLayer>, String> {
        let shadows = context.settings.shadows;

        RenderChain::new([0, 53, 116, 255], context.common_settings.antialias, |r| {
            r.enable_cull_face(); // Required for shadows.
            ShadowLayer::with_viewport(
                r,
                Mk48Layer {
                    // TODO when recreated with animations turned off can cause issues.
                    background: Mk48BackgroundLayer::new(
                        r,
                        context.settings.animations,
                        context.settings.dynamic_waves,
                        shadows,
                    ),
                    sea_level_particles: Mk48ParticleLayer::new(r, shadows),
                    sprites: SpriteLayer::new(r, shadows),
                    airborne_particles: Mk48ParticleLayer::new(r, shadows),
                    trails: TrailLayer::new(r),
                    overlay: Mk48OverlayLayer::new(r),
                    graphics: GraphicLayer::new(r),
                    text: TextLayer::new(r),
                },
                match shadows {
                    ShadowSetting::None => None,
                    ShadowSetting::Hard => Some(UVec2::splat(2048)),
                    ShadowSetting::Soft => Some(UVec2::splat(512)),
                },
            )
        })
    }
}

impl GameClient for Mk48Game {
    const GAME_ID: GameId = GameId::Mk48;
    const LICENSES: &'static [(&'static str, &'static [&'static str])] = crate::licenses::LICENSES;

    type Audio = Audio;
    type GameRequest = Command;
    type GameState = Mk48State;
    type UiEvent = UiEvent;
    type UiProps = UiProps;
    type GameUpdate = Update;
    type GameSettings = Mk48Settings;

    fn new(context: &Context<Self>) -> Result<Self, String> {
        let ui_props_rate_limiter = RateLimiter::new(0.1);

        let render_chain = Self::create_render_chain(context)?;
        let camera = Camera2d::default();
        let mk48_camera = Mk48Camera::default();

        Ok(Self {
            mk48_camera,
            camera,
            render_chain,
            first_control: false,
            first_zoom: false,
            holding: false,
            reversing: false,
            interpolated_altitude: Interpolated::new(0.2),
            respawn_overridden: false,
            last_control: None,
            control_rate_limiter: RateLimiter::new(0.1),
            ui_props_rate_limiter,
            alarm_fast_rate_limiter: RateLimiter::new(10.0),
            peek_update_sound_counter: 0,
            fire_rate_limiter: FireRateLimiter::new(),
            fps_counter: FpsMonitor::new(1.0),
            ui_state: UiState::default(),
        })
    }

    /// This violates the normal "peek" contract by doing the work of apply, when it comes to contacts.
    fn peek_game(&mut self, update: &Update, context: &mut Context<Self>) {
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
                        self.camera.center,
                        &*context,
                        &context.audio,
                    );
                }
                if contact.player_id() == context.state.core.player_id && contact.is_boat() {
                    context.state.game.entity_id = Some(contact.id());
                    // Just spawned so reset these.
                    self.first_control = true;
                    self.first_zoom = true;
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
                let time_seconds = context.client.time_seconds;
                self.play_lost_contact_audio_and_animations(
                    self.camera.center,
                    &contact,
                    &context.audio,
                    &mut context.state.game.animations,
                    time_seconds,
                );
            }
        }

        let player_position = self.camera.center;
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

    fn peek_keyboard(&mut self, event: &KeyboardEvent, context: &mut Context<Self>) {
        if event.down {
            if let Some(contact) = context.state.game.player_contact() {
                let entity_type = contact.entity_type().unwrap();
                let consumptions: Vec<bool> = contact.reloads().iter().map(|b| *b).collect();
                let groups = group_armaments(&entity_type.data().armaments, &consumptions);
                match event.key {
                    SURFACE_KEY => {
                        self.set_submerge(!self.ui_state.submerge, &*context);
                    }
                    ACTIVE_KEY => {
                        self.set_active(!self.ui_state.active, &*context);
                    }
                    Key::Tab => {
                        self.ui_state.armament = groups
                            .get(
                                self.ui_state
                                    .armament
                                    .and_then(|current| {
                                        groups.iter().position(|Group { entity_type, .. }| {
                                            *entity_type == current
                                        })
                                    })
                                    .map(|idx| (groups.len() + idx + 1) % groups.len())
                                    .unwrap_or(0),
                            )
                            .map(|Group { entity_type, .. }| *entity_type);
                    }
                    _ => {
                        if let Some(digit) = event.key.digit_with_ten() {
                            if let Some(armament) = groups
                                .get((digit.get() - 1) as usize)
                                .map(|Group { entity_type, .. }| *entity_type)
                            {
                                self.ui_state.armament = Some(armament);
                            }
                        }
                    }
                }
            }
        }
    }

    fn peek_mouse(&mut self, event: &MouseEvent, _context: &mut Context<Self>) {
        if let MouseEvent::Wheel(delta) = event {
            self.mk48_camera.zoom(*delta);
            self.first_zoom = false;
        }
    }

    fn tick(&mut self, elapsed_seconds: f32, context: &mut Context<Self>) {
        let mut frame = self.render_chain.begin(context.client.time_seconds);
        let (renderer, shadow_layer) = frame.draw();
        let layer = &mut shadow_layer.inner;

        // Allow more sounds to be played in peek.
        self.peek_update_sound_counter = 0;

        // The distance from player's boat to the closest visible member of each team, for the purpose of sorting and
        // filtering.
        let mut team_proximity: HashMap<TeamId, f32> = HashMap::new();

        // Temporary (will be recalculated after moving ships).
        self.mk48_camera.update(
            context.state.game.player_contact(),
            elapsed_seconds,
            layer.background.cache_frame,
        );
        let (camera, _) = self
            .mk48_camera
            .camera(context.state.game.player_contact(), renderer.aspect_ratio());

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
        let (camera, zoom) = self
            .mk48_camera
            .camera(context.state.game.player_contact(), renderer.aspect_ratio());

        // Set camera before update layers so they don't get last frame's camera.
        // TODO decouple update and render.
        self.camera.update(camera, zoom, renderer.canvas_size());
        let weather = Weather::new(renderer.time);

        let (visual_range, visual_restriction, area) =
            if let Some(c) = context.state.game.player_interpolated_contact() {
                // Use model as input to interpolation (can't interpolate twice).
                let altitude = c.model.altitude().to_norm();
                let t = context.client.time_seconds;
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
        sortable_sprites.extend(layer.background.update(
            camera,
            zoom,
            &mut context.state.game.terrain,
            terrain_reset,
            context.settings.shadows.is_some(),
            &*renderer,
        ));
        shadow_layer.camera = layer.background.shadow_camera(
            renderer,
            &self.camera,
            &weather,
            context.settings.shadows,
        );

        layer.overlay.update(
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

            if animation.frame(context.client.time_seconds) >= len {
                context.state.game.animations.swap_remove(i);
            } else {
                sortable_sprites.push(SortableSprite::new_animation(
                    animation,
                    context.client.time_seconds,
                ));
                i += 1;
            }
        }

        // Update trails.
        layer.trails.set_time(context.client.time_seconds);

        for InterpolatedContact { view: contact, .. } in context.state.game.contacts.values() {
            let friendly = context.state.core.is_friendly(contact.player_id());

            let color_bytes = if friendly {
                [58, 255, 140]
            } else if contact.is_boat() {
                [255; 3]
            } else {
                [231, 76, 60]
            };
            let color = rgb_array(color_bytes);

            if let Some(entity_type) = contact.entity_type() {
                let altitude = contact.altitude().to_meters();
                // Only boats have non linear altitude to alpha (because they have more depth).
                // TODO give entities depth dimension.
                let alpha = if !contact.is_boat() || contact.altitude().is_submerged() {
                    let max_alpha = if contact.is_boat() { 0.7 } else { 1.0 };
                    map_ranges(
                        contact.altitude().to_norm(),
                        -1.0..0.0,
                        0.2..max_alpha,
                        true,
                    )
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
                        && settings.dynamic_waves
                        && data.kind == EntityKind::Collectible
                    {
                        let t = context.client.time_seconds;

                        // Moves waves with the wind.
                        let mut input = (transform.position + weather.wind * t) * 0.1;

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
                        self.camera.center,
                        &mut layer.airborne_particles,
                    );
                }

                if contact.is_boat() {
                    for i in 0..data.armaments.len() {
                        let armament = &data.armaments[i];
                        if armament.hidden
                            || armament.vertical
                            || !(armament.external || (friendly && !context.settings.cinematic))
                        {
                            continue;
                        }
                        let armament_type = armament.entity_type;

                        let reloaded = contact.reloads().get(i).map(|r| *r).unwrap_or(false);
                        if !reloaded && context.settings.cinematic {
                            continue;
                        }

                        sortable_sprites.push(SortableSprite::new_child_entity(
                            entity_id,
                            entity_type,
                            armament_type,
                            *contact.transform() + data.armament_transform(contact.turrets(), i),
                            altitude + 0.02,
                            alpha * if reloaded { 1.0 } else { 0.5 },
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

                if !context.settings.cinematic {
                    match data.kind {
                        EntityKind::Boat => {
                            // Is this player's own boat?
                            if context.state.core.player_id.is_some()
                                && contact.player_id() == context.state.core.player_id
                            {
                                // Radii
                                let hud_color = gray_a(255, 50);
                                let reverse_color = rgba(255, 75, 75, 75);
                                let hud_thickness = 0.0025 * zoom;

                                if context.settings.circle_hud {
                                    // Throttle rings.
                                    // 1. Inner
                                    layer.graphics.draw_circle(
                                        contact.transform().position,
                                        data.radii().start,
                                        hud_thickness,
                                        hud_color,
                                    );
                                    // 2. Outer
                                    layer.graphics.draw_circle(
                                        contact.transform().position,
                                        data.radii().end,
                                        hud_thickness,
                                        hud_color,
                                    );
                                    // 3. Actual speed
                                    layer.graphics.draw_circle(
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
                                    let velocity_target = contact
                                        .guidance()
                                        .velocity_target
                                        .max(data.speed * Velocity::MAX_REVERSE_SCALE);
                                    layer.graphics.draw_circle(
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
                                if context.settings.circle_hud
                                    || Self::is_holding_control(
                                        &context.mouse,
                                        context.client.time_seconds,
                                    )
                                {
                                    let guidance = contact.guidance();
                                    let mut direction = guidance.direction_target;
                                    let mut color = hud_color;

                                    // Is reversing.
                                    // Fix ambiguity when loading guidance from server.
                                    if guidance.velocity_target < Velocity::ZERO
                                        || guidance.velocity_target == Velocity::ZERO
                                            && self.reversing
                                    {
                                        // Flip dir if going backwards.
                                        direction += Angle::PI;
                                        // Reversing color.
                                        color = reverse_color;
                                        // TODO BEEP BEEP BEEP
                                    }

                                    let angle = direction.to_radians();
                                    let dir_mat = Mat2::from_angle(angle);
                                    let radii = data.radii();

                                    // Don't overlap inner/outer hud circles.
                                    let overlap = hud_thickness
                                        * context.settings.circle_hud as u8 as f32
                                        * 0.5;
                                    let start = contact.transform().position
                                        + dir_mat * Vec2::new(radii.start + overlap, 0.0);
                                    let mut end = contact.transform().position
                                        + dir_mat * Vec2::new(radii.end - overlap, 0.0);
                                    let mut line_width = hud_thickness;

                                    if !context.settings.circle_hud {
                                        let velocity_target = contact
                                            .guidance()
                                            .velocity_target
                                            .max(data.speed * Velocity::MAX_REVERSE_SCALE);
                                        let t =
                                            velocity_target.abs().to_mps() / data.speed.to_mps();

                                        // Make sure scale is never too big or too small.
                                        let mut scale = (hud_thickness * 2.0)
                                            .min(radii.start * 0.03)
                                            .max(hud_thickness);
                                        let new_scale = scale * 2.5;

                                        let diff = end - start;
                                        let len = diff.length();
                                        if len * t < new_scale {
                                            scale = new_scale;
                                        }

                                        let scale_t = scale / len;
                                        let center =
                                            start.lerp(end, (t - scale_t * 0.5).max(scale_t * 0.5));
                                        end = start + diff * (t - scale_t).max(0.0);

                                        layer.graphics.draw_triangle(
                                            center,
                                            Vec2::splat(scale),
                                            angle - PI * 0.5,
                                            color,
                                        );
                                        line_width = scale;
                                    }

                                    layer.graphics.draw_line(start, end, line_width, color);
                                }

                                // Reverse azimuths.
                                if context.settings.circle_hud && Self::has_reverse(contact) {
                                    let mut range = data.radii();
                                    // Don't overlap inner hud circle.
                                    range.start += hud_thickness * 0.5;
                                    range.end = lerp(range.start, range.end, 0.1);

                                    let position = contact.transform().position;
                                    let direction = contact.transform().direction.to_radians() - PI;
                                    let mut color = hud_color;

                                    if self.holding
                                        || !Self::can_reverse(self.first_control, contact)
                                    {
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

                                        layer.graphics.draw_line_gradient(
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
                                let ui_armament = self.ui_state.armament;
                                if let Some((i, mouse_pos)) =
                                    context.mouse.view_position.and_then(|view_pos| {
                                        let mouse_pos = self.camera.to_world_position(view_pos);
                                        Self::find_best_armament(
                                            &self.fire_rate_limiter,
                                            contact,
                                            false,
                                            mouse_pos,
                                            ui_armament,
                                        )
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
                                            let transform = transform
                                                + Transform::from_position(turret.position());

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

                                                layer.graphics.draw_line(
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
                                                layer.graphics.draw_arc(
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
                                                layer.graphics.draw_arc(
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

                                                layer.graphics.draw_triangle(
                                                    center,
                                                    scale,
                                                    angle_radians - PI * 0.5,
                                                    color,
                                                );
                                            } else {
                                                let inner: f32 = 0.11 * data.width;
                                                layer.graphics.draw_circle(
                                                    transform.position,
                                                    inner,
                                                    inner * 0.55,
                                                    color,
                                                )
                                            }

                                            if is_vertical {
                                                let inner: f32 = 0.055 * data.width;
                                                layer.graphics.draw_filled_circle(
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
                                layer.graphics.draw_rounded_line(
                                    offset_x(center, length * -0.5),
                                    offset_x(center, length * 0.5),
                                    thickness,
                                    bg_color,
                                    true,
                                );

                                // Health indicator.
                                layer.graphics.draw_rounded_line(
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

                            let c = color_bytes;
                            layer.text.draw(
                                &text,
                                contact.transform().position
                                    + Vec2::new(0.0, overlay_vertical_position + 0.035 * zoom),
                                0.035 * zoom,
                                [c[0], c[1], c[2], 255],
                            );
                        }
                        EntityKind::Weapon | EntityKind::Decoy | EntityKind::Aircraft => {
                            let triangle_position = contact.transform().position
                                + Vec2::new(0.0, overlay_vertical_position);
                            layer.graphics.draw_triangle(
                                triangle_position + Vec2::new(0.0, 0.01 * zoom),
                                Vec2::splat(0.02 * zoom),
                                180f32.to_radians(),
                                color.extend(1.0),
                            );
                        }
                        _ => {}
                    }
                }

                // Add particles.
                let mut rng = thread_rng();
                let direction_vector: Vec2 = contact.transform().direction.into();
                let tangent_vector = direction_vector.perp();
                let speed = contact.transform().velocity.to_mps();

                // Integer amount of particles from fractional per_second
                let amount = {
                    let per_second = data.width * 6.0 + speed * 2.0;
                    let t = context.client.time_seconds;
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
                        layer.trails.add_trail(
                            entity_id,
                            t.position,
                            t.direction.to_vec() * t.velocity.to_mps(),
                            data.width * 2.0,
                        );
                    } else {
                        let is_airborne = contact.altitude().is_airborne();
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

                            let particle = Mk48Particle {
                                position,
                                velocity,
                                radius: 1.0,
                                color: 1.0,
                                smoothness: 1.0,
                            };

                            if is_airborne {
                                layer.airborne_particles.add(particle);
                            } else {
                                layer.sea_level_particles.add(particle);
                            }
                        }

                        // Side wake.
                        if data.kind == EntityKind::Boat
                            && data.sub_kind != EntitySubKind::Submarine
                            && contact.altitude() == Altitude::ZERO
                        {
                            for _ in 0..amount * 2 {
                                let r = rng.gen::<f32>() - 0.6;
                                let side = if rng.gen() { -1f32 } else { 1f32 };

                                let position = start
                                    + direction_vector * (data.length * r * 0.5)
                                    + tangent_vector * (data.width * side * 0.3);

                                let velocity = direction_vector * (speed * 0.1)
                                    + tangent_vector
                                        * ((spread * 1.0 + data.width * 0.01) * speed * side);

                                layer.sea_level_particles.add(Mk48Particle {
                                    position,
                                    velocity,
                                    radius: 1.0,
                                    color: 1.0,
                                    smoothness: 1.0,
                                });
                            }
                        }
                    }
                }

                // Exhaust particles
                if !contact.altitude().is_submerged() {
                    for exhaust in data.exhausts.iter() {
                        for _ in 0..amount * 2 {
                            layer.airborne_particles.add(Mk48Particle {
                                position: contact.transform().position
                                    + direction_vector * exhaust.position_forward
                                    + tangent_vector * exhaust.position_side
                                    + gen_radius(&mut rng, 1.5),
                                velocity: gen_radius(&mut rng, 6.0),
                                radius: 1.0,
                                color: if entity_type == EntityType::OilPlatform {
                                    -1.0
                                } else {
                                    0.23
                                },
                                smoothness: 1.0,
                            });
                        }
                    }
                }
            } else {
                layer.sprites.draw(
                    "contact",
                    None,
                    contact.transform().position,
                    Vec2::splat(10.0),
                    contact.transform().direction.to_radians(),
                    1.0,
                    0.0,
                    0.0,
                );
            }
        }

        // Play anti-aircraft sfx.
        if anti_aircraft_volume > 0.0 && !context.audio.is_playing(Audio::Aa) {
            context
                .audio
                .play_with_volume(Audio::Aa, anti_aircraft_volume.min(0.5));
        }

        // Sort sprites by altitude.
        sortable_sprites.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        for SortableSprite {
            alpha,
            altitude,
            dimensions,
            frame,
            height,
            shadow_height,
            sprite,
            transform:
                Transform {
                    direction,
                    position,
                    ..
                },
            ..
        } in sortable_sprites
        {
            let angle = direction.to_radians();
            if alpha == 1.0 && shadow_height != 0.0 && context.settings.shadows.is_some() {
                // Water plane covers whole world and is pointing up.
                let water_pos = Vec3::ZERO;
                let water_normal = Vec3::Z;

                // Cast shadow ray.
                let shadow_pos = position.extend(shadow_height);
                let shadow_normal = -weather.sun;

                // Cast ray from ship's center along sun's direction towards water to get shadow
                // offset.
                let water_distance = water_normal.dot(shadow_pos) - water_normal.dot(water_pos);
                if water_distance > 0.0 {
                    let normal_dot = -shadow_normal.dot(water_normal);
                    if normal_dot > 0.0 {
                        let scale = water_distance / normal_dot;
                        let displacement = shadow_normal * scale;
                        let center = (shadow_pos + displacement).truncate();

                        // let center = position - Vec2::splat(0.2) * height;
                        layer
                            .sprites
                            .draw_shadow(sprite, frame, center, dimensions, angle);
                    }
                }
            }

            let center = position;
            layer.sprites.draw(
                sprite, frame, center, dimensions, angle, alpha, altitude, height,
            );
        }

        // For hinting to server.
        let aspect_ratio = renderer.aspect_ratio();
        frame.end(&Mk48Params {
            camera: self.camera.clone(),
            weather,
        });

        // After the above line, mouse world position state may be out-of-date. Recalculate it here.
        let aim_target = context
            .mouse
            .view_position
            .map(|p| self.camera.to_world_position(p));

        // Send command later, when lifetimes allow.
        let mut control: Option<Command> = None;

        let player_contact = Self::maybe_contact_mut(
            &mut context.state.game.contacts,
            context.state.game.entity_id,
        );

        crate::armament::update(
            player_contact.as_ref().and_then(|c| c.model.entity_type()),
            &mut self.ui_state.armament,
        );

        let status = if let Some(player_contact) = player_contact {
            let mut guidance = None;

            {
                let player_contact = &player_contact.view;
                let max_speed = player_contact.data().speed.to_mps();

                let joystick = Joystick::try_from_keyboard_state(
                    context.client.time_seconds,
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

                if Self::is_holding_control(&context.mouse, context.client.time_seconds) {
                    let current_dir = player_contact.transform().direction;
                    let mut direction_target = Angle::from(
                        aim_target.unwrap_or_default() - player_contact.transform().position,
                    );

                    // Only do when start holding.
                    if !self.holding {
                        if Self::can_reverse(self.first_control, player_contact) {
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

            let status = UiStatus::Playing(UiStatusPlaying {
                entity_type: player_contact.entity_type().unwrap(),
                position: player_contact.transform().position.into(),
                direction: player_contact.transform().direction,
                velocity: player_contact.transform().velocity,
                altitude: player_contact.altitude(),
                submerge: self.ui_state.submerge,
                active: self.ui_state.active,
                instruction_status: if player_contact.data().level <= 3 {
                    InstructionStatus {
                        touch: context.mouse.touch_screen,
                        basics: self.first_control,
                        zoom: self.first_zoom,
                    }
                } else {
                    InstructionStatus::default()
                },
                armament: self.ui_state.armament,
                armament_consumption: player_contact.reloads().iter().map(|b| *b).collect(),
                team_proximity,
            });

            if self.control_rate_limiter.update_ready(elapsed_seconds) {
                let left_click = context.mouse.take_click(MouseButton::Left);

                // Get hint before borrow of player_contact().
                let hint = Some(Hint {
                    aspect: aspect_ratio,
                });

                let current_control = Control {
                    guidance: Some(*player_contact.guidance()), // TODO don't send if hasn't changed.
                    submerge: self.ui_state.submerge,
                    aim_target,
                    active: self.ui_state.active,
                    pay: context.keyboard.is_down(Key::C).then_some(Pay),
                    fire: if left_click
                        || context
                            .keyboard
                            .state(Key::Space)
                            .combined(context.keyboard.state(Key::E))
                            .is_down()
                    {
                        Self::find_best_armament(
                            &self.fire_rate_limiter,
                            player_contact,
                            true,
                            aim_target.unwrap_or_default(),
                            self.ui_state.armament,
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
        } else if let Some(death_reason) = context
            .state
            .game
            .death_reason
            .as_ref()
            .filter(|_| !self.respawn_overridden)
            .cloned()
        {
            UiStatus::Respawning(UiStatusRespawning { death_reason })
        } else {
            UiStatus::Spawning
        };

        if let Some(control) = control {
            context.send_to_game(control);
        }

        self.fps_counter.update(elapsed_seconds);
        self.fire_rate_limiter.update(elapsed_seconds);

        if self.ui_props_rate_limiter.update_ready(elapsed_seconds) {
            self.update_ui_props(context, status);
        }
    }

    fn ui(&mut self, event: UiEvent, context: &mut Context<Self>) {
        match event {
            UiEvent::Active(active) => {
                self.set_active(active, &*context);
            }
            UiEvent::Armament(armament) => {
                self.ui_state.armament = armament;
            }
            UiEvent::GraphicsSettingsChanged => {
                self.render_chain = Self::create_render_chain(context).unwrap();
            }
            UiEvent::OverrideRespawn => {
                self.respawn_overridden = true;
            }
            UiEvent::Respawn(entity_type) => {
                context.send_to_game(Command::Spawn(Spawn { entity_type }));
            }
            UiEvent::Spawn { alias, entity_type } => {
                context.send_set_alias(alias);
                context.send_to_game(Command::Spawn(Spawn { entity_type }));
            }
            UiEvent::Submerge(submerge) => {
                self.set_submerge(submerge, &*context);
            }
            UiEvent::Upgrade(entity_type) => {
                context.audio.play(Audio::Upgrade);
                context.send_to_game(Command::Upgrade(Upgrade { entity_type }));
            }
        }
    }
}

impl Mk48Game {
    fn set_active(&mut self, active: bool, context: &Context<Self>) {
        if let Some(contact) = context.state.game.player_contact() {
            if active && contact.data().sensors.sonar.range >= 0.0 {
                context.audio.play(Audio::Sonar1);
            }
        }
        self.ui_state.active = active;
    }

    fn set_submerge(&mut self, submerge: bool, context: &Context<Self>) {
        if let Some(contact) = context.state.game.player_contact() {
            if contact.data().sub_kind == EntitySubKind::Submarine {
                if !self.ui_state.submerge && submerge {
                    context.audio.play(Audio::Dive);
                } else if self.ui_state.submerge && !submerge {
                    context.audio.play(Audio::Surface);
                }
            }
        }
        self.ui_state.submerge = submerge;
    }
}
