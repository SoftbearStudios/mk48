// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::background::{Mk48BackgroundContext, Mk48OverlayContext};
use crate::interpolated_contact::InterpolatedContact;
use crate::settings::Mk48Settings;
use crate::sprite::SortableSprite;
use crate::state::Mk48State;
use crate::ui::{DeathReasonModel, UiEvent, UiProps, UiState, UiStatus};
use client_util::audio::AudioLayer;
use client_util::context::Context;
use client_util::fps_monitor::FpsMonitor;
use client_util::game_client::GameClient;
use client_util::joystick::Joystick;
use client_util::keyboard::Key;
use client_util::mouse::{MouseButton, MouseEvent};
use client_util::rate_limiter::RateLimiter;
use client_util::renderer::background::BackgroundLayer;
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
use common::velocity::Velocity;
use common_util::range::{gen_radius, map_ranges};
use core_protocol::id::{GameId, TeamId};
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::ClientRequest;
use glam::{Mat2, UVec2, Vec2, Vec4};
use rand::{thread_rng, Rng};
use std::collections::HashMap;

pub struct Mk48Game {
    /// Holding mouse down, for the purpose of deciding whether to start reversing.
    pub holding: bool,
    /// Currently in mouse control reverse mode.
    pub reversing: bool,
    /// Camera on death.
    pub saved_camera: Option<(Vec2, f32)>,
    /// In meters.
    pub interpolated_zoom: f32,
    /// 1 = normal.
    pub zoom_input: f32,
    /// Rate limit control websocket messages.
    pub control_rate_limiter: RateLimiter,
    /// Rate limit ui props messages.
    pub ui_props_rate_limiter: RateLimiter,
    /// Playing the alarm fast sound too often is annoying.
    pub alarm_fast_rate_limiter: RateLimiter,
    /// FPS counter
    pub fps_counter: FpsMonitor,
}

/// Order of fields is order of rendering.
#[derive(Layer)]
pub struct RendererLayer {
    pub audio: AudioLayer,
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

impl GameClient for Mk48Game {
    const GAME_ID: GameId = GameId::Mk48;

    type Command = Command;
    type RendererLayer = RendererLayer;
    type State = Mk48State;
    type UiEvent = UiEvent;
    type UiState = UiState;
    type UiProps = UiProps;
    type Update = Update;
    type Settings = Mk48Settings;

    fn new() -> Self {
        unsafe {
            // SAFETY: First thing to run, happens before any entity data loading.
            EntityType::init();
        }

        Self {
            holding: false,
            reversing: false,
            interpolated_zoom: Self::DEFAULT_ZOOM_INPUT * Self::MENU_VISUAL_RANGE,
            zoom_input: Self::DEFAULT_ZOOM_INPUT,
            saved_camera: None,
            control_rate_limiter: RateLimiter::new(0.1),
            ui_props_rate_limiter: RateLimiter::new(0.1),
            alarm_fast_rate_limiter: RateLimiter::new(10.0),
            fps_counter: FpsMonitor::new(5.0),
        }
    }

    fn init(
        &mut self,
        renderer: &mut Renderer,
        context: &mut Context<Self>,
    ) -> Self::RendererLayer {
        renderer.set_background_color(Vec4::new(0.0, 0.20784314, 0.45490196, 1.0));

        let background_frag_template = include_str!("./shaders/background.frag");
        let mut background_frag_source =
            String::with_capacity(background_frag_template.len() + 100);

        if context.settings.wave_quality != 0 {
            renderer.enable_oes_standard_derivatives();
            background_frag_source +=
                &*format!("#define WAVES {}\n", context.settings.wave_quality * 2);
        }

        if !context.settings.render_terrain_textures {
            fn vec3_glsl(c: [u8; 3]) -> String {
                let [r, g, b] = c;
                let v = rgb(r, g, b);
                format!("vec3({:.2}, {:.2}, {:.2})", v.x, v.y, v.z)
            }

            background_frag_source += &*format!(
                "#define SAND_COLOR vec3({})\n",
                vec3_glsl(Mk48BackgroundContext::SAND_COLOR)
            );
            background_frag_source += &*format!(
                "#define GRASS_COLOR vec3({})\n",
                vec3_glsl(Mk48BackgroundContext::GRASS_COLOR)
            );
        }

        background_frag_source += background_frag_template;

        let background_shader = renderer.create_shader(
            include_str!("./shaders/background.vert"),
            &background_frag_source,
        );

        let overlay_shader = renderer.create_shader(
            include_str!("./shaders/overlay.vert"),
            include_str!("./shaders/overlay.frag"),
        );

        let audio_sprite_sheet =
            serde_json::from_str(include_str!("./sprites_audio.json")).unwrap();
        let sprite_sheet = serde_json::from_str(include_str!("./sprites_webgl.json")).unwrap();
        let sprite_texture =
            renderer.load_texture("/sprites_webgl.png", UVec2::new(2048, 2048), None, false);

        let background_context =
            Mk48BackgroundContext::new(context.settings.render_terrain_textures, &*renderer);

        let overlay_context = Mk48OverlayContext::default();

        RendererLayer {
            audio: AudioLayer::new("/sprites_audio.mp3", audio_sprite_sheet),
            background: BackgroundLayer::new(renderer, background_shader, background_context),
            sea_level_particles: ParticleLayer::new(renderer, Vec2::ZERO),
            sprites: SpriteLayer::new(renderer, sprite_texture, sprite_sheet),
            airborne_particles: ParticleLayer::new(renderer, wind()),
            airborne_graphics: GraphicLayer::new(renderer),
            overlay: BackgroundLayer::new(renderer, overlay_shader, overlay_context),
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
        layer: &mut Self::RendererLayer,
    ) {
        let updated: HashMap<EntityId, &Contact> =
            update.contacts.iter().map(|c| (c.id(), c)).collect();

        for (id, &contact) in updated.iter() {
            if let Some(InterpolatedContact { model, .. }) = context.game().contacts.get(id) {
                if Some(*id) == context.game().entity_id {
                    let recent_damage = contact.damage().saturating_sub(model.damage());
                    if recent_damage > Ticks::ZERO {
                        layer.audio.play("damage");

                        // Considered "intense" 250% of the damage would have been fatal.
                        if recent_damage * 2.5
                            >= model.data().max_health().saturating_sub(model.damage())
                        {
                            Self::play_music("intense", &layer.audio);
                        }
                    }
                }

                // Mutable borrow after immutable borrows.
                let network_contact = context.game_mut().contacts.get_mut(id).unwrap();
                network_contact.model = contact.clone();

                // Compensate for the fact that the data is a little old (second parameter is rough
                // estimate of latency)
                network_contact.model.simulate(0.1);
            } else {
                self.new_contact(contact, renderer.camera_center(), &*context, &layer.audio);
                if contact.player_id() == context.core().player_id && contact.is_boat() {
                    context.game_mut().entity_id = Some(contact.id());
                }
                context
                    .game_mut()
                    .contacts
                    .insert(contact.id(), InterpolatedContact::new(contact.clone()));
            }
        }

        // Contacts absent in the update are currently considered lost.
        // Borrow entity_id early to avoid use of self in closure.
        let game_state = context.game_mut();
        let entity_id = &mut game_state.entity_id;
        for contact in game_state
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
            let time_seconds = context.client.update_seconds;
            self.lost_contact(
                renderer.camera_center(),
                &contact,
                &layer.audio,
                &mut context.game_mut().animations,
                time_seconds,
            );
        }

        let player_position = renderer.camera_center();
        let player_altitude = context
            .game()
            .player_contact()
            .map(|c| c.altitude())
            .unwrap_or(Altitude::ZERO);

        let mut aircraft_volume: f32 = 0.0;
        let mut jet_volume: f32 = 0.0;
        let mut need_to_dodge: f32 = 0.0;

        for (_, InterpolatedContact { view: contact, .. }) in context.game().contacts.iter() {
            if let Some(entity_type) = contact.entity_type() {
                let data: &'static EntityData = entity_type.data();
                let position_diff = contact.transform().position - player_position;
                let direction = Angle::from(position_diff);
                let distance = position_diff.length();
                let inbound =
                    (contact.transform().direction - direction + Angle::PI).abs() < Angle::PI_2;

                let friendly = context.core().is_friendly(contact.player_id());
                let volume = Self::volume_at(distance);

                if data.kind == EntityKind::Aircraft {
                    if matches!(entity_type, EntityType::SuperEtendard) {
                        jet_volume += volume;
                    } else {
                        aircraft_volume += volume;
                    }
                }

                if context.game().entity_id.is_some() && distance < 250.0 {
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
            layer
                .audio
                .play_with_volume("aircraft", (aircraft_volume + 1.0).ln());
        }

        if jet_volume > 0.01 {
            layer.audio.play_with_volume("jet", (jet_volume + 1.0).ln());
        }

        if need_to_dodge >= 3.0 {
            Self::play_music("dodge", &layer.audio);
        }

        let score_delta = update.score.saturating_sub(context.game().score);
        if score_delta >= 10
            && (score_delta >= 200 || score_delta as f32 / context.game().score as f32 > 0.5)
        {
            Self::play_music("achievement", &layer.audio);
        }
    }

    fn peek_mouse(&mut self, event: &MouseEvent, _context: &mut Context<Self>) {
        if let MouseEvent::Wheel(delta) = event {
            self.zoom(delta);
        }
    }

    fn tick(
        &mut self,
        elapsed_seconds: f32,
        context: &mut Context<Self>,
        renderer: &mut Renderer,
        layer: &mut Self::RendererLayer,
    ) {
        // Don't create more particles at 144hz.
        let particle_multiplier = elapsed_seconds.min(1.0 / 60.0) * 60.0;

        layer.audio.set_volume(context.settings.volume);
        if !layer.audio.is_playing("ocean") {
            layer.audio.play_looping("ocean");
        }

        // The distance from player's boat to the closest visible member of each team, for the purpose of sorting and
        // filtering.
        let mut team_proximity: HashMap<TeamId, f32> = HashMap::new();

        // Temporary (will be recalculated after moving ships).
        self.update_camera(context.game().player_contact(), elapsed_seconds);
        let (camera, _) = self.camera(context.game().player_contact(), renderer.aspect_ratio());

        // Cannot borrow entire context, do this instead.
        let connection_lost = context.game_connection_lost();
        let game_state = context.game_socket.as_mut().unwrap().state_mut();
        let core_state = context.core_socket.state();

        let debug_latency_entity_id = if false { game_state.entity_id } else { None };
        // A subset of game logic.
        for interp in &mut game_state.contacts.values_mut() {
            if interp
                .model
                .entity_type()
                .map(|e| e.data().kind == EntityKind::Boat)
                .unwrap_or(false)
            {
                // Update team_proximity.
                if let Some(player_id) = interp.model.player_id() {
                    if let Some(player) = core_state.only_players().get(&player_id) {
                        if let Some(team_id) = player.team_id {
                            let distance =
                                camera.distance_squared(interp.model.transform().position);
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
            interp.interpolate(elapsed_seconds, game_state.entity_id);
        }

        // May have changed due to the above.
        let (camera, zoom) = self.camera(game_state.player_contact(), renderer.aspect_ratio());

        let (visual_range, visual_restriction) =
            if let Some(player_contact) = game_state.player_contact() {
                let alt_norm = player_contact.altitude().to_norm();
                (
                    player_contact
                        .entity_type()
                        .unwrap()
                        .data()
                        .sensors
                        .visual
                        .range
                        * map_ranges(alt_norm, -1.0..0.0, 0.4..0.8, true),
                    map_ranges(
                        player_contact.altitude().to_norm(),
                        0.0..-1.0,
                        0.0..0.8,
                        true,
                    ),
                )
            } else {
                (500.0, 0.0)
            };

        // Prepare to sort sprites.
        let mut sortable_sprites = Vec::with_capacity(game_state.contacts.len() * 5);

        // Update background and add vegetation sprites.
        sortable_sprites.extend(layer.background.context.update(
            camera,
            zoom,
            context.client.update_seconds,
            &game_state.terrain,
            &*renderer,
        ));

        layer
            .overlay
            .context
            .update(visual_range, visual_restriction, game_state.world_radius);

        let mut anti_aircraft_volume = 0.0;

        // Update animations.
        let mut i = 0;
        while i < game_state.animations.len() {
            let animation = &mut game_state.animations[i];

            let len = layer.sprites.animation_length(animation.name);

            if animation.frame(context.client.update_seconds) >= len {
                game_state.animations.swap_remove(i);
            } else {
                sortable_sprites.push(SortableSprite::new_animation(
                    animation,
                    context.client.update_seconds,
                ));
                i += 1;
            }
        }

        // Update trails.
        game_state.trails.set_time(context.client.update_seconds);

        for InterpolatedContact { view: contact, .. } in game_state.contacts.values() {
            let friendly = core_state.is_friendly(contact.player_id());

            let color = if friendly {
                rgb(58, 255, 140)
            } else if contact.is_boat() {
                gray(255)
            } else {
                rgb(231, 76, 60)
            };

            if let Some(entity_type) = contact.entity_type() {
                let altitude = contact.altitude().to_norm();
                let alpha = (altitude + 1.0).clamp(0.25, 1.0);
                let entity_id = contact.id();

                sortable_sprites.push(SortableSprite::new_entity(
                    entity_id,
                    entity_type,
                    *contact.transform(),
                    altitude,
                    alpha,
                ));

                let data: &'static EntityData = entity_type.data();
                let parent_type = entity_type;

                if contact.is_boat()
                    && !contact.altitude().is_submerged()
                    && data.anti_aircraft > 0.0
                {
                    anti_aircraft_volume += Self::simulate_anti_aircraft(
                        contact,
                        &game_state.contacts,
                        core_state,
                        renderer.camera_center(),
                        &mut layer.airborne_particles,
                    );
                }

                if contact.is_boat() && !contact.reloads().is_empty() {
                    for i in 0..data.armaments.len() {
                        let armament = &data.armaments[i];
                        if armament.hidden || armament.vertical || !(armament.external || friendly)
                        {
                            continue;
                        }
                        sortable_sprites.push(SortableSprite::new_child_entity(
                            entity_id,
                            parent_type,
                            armament.entity_type,
                            *contact.transform() + data.armament_transform(contact.turrets(), i),
                            altitude + 0.02,
                            alpha
                                * (if contact.reloads()[i] == Ticks::ZERO {
                                    1.0
                                } else {
                                    0.5
                                }),
                        ));
                    }
                }
                for (i, turret) in data.turrets.iter().enumerate() {
                    if let Some(turret_type) = turret.entity_type {
                        let pos = turret.position();
                        sortable_sprites.push(SortableSprite::new_child_entity(
                            entity_id,
                            parent_type,
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
                        if core_state.player_id.is_some()
                            && contact.player_id() == core_state.player_id
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
                            layer.graphics.add_circle(
                                contact.transform().position,
                                map_ranges(
                                    contact.guidance().velocity_target.abs().to_mps(),
                                    0.0..data.speed.to_mps(),
                                    data.radii(),
                                    true,
                                ),
                                hud_thickness,
                                if contact.guidance().velocity_target < Velocity::ZERO {
                                    reverse_color
                                } else {
                                    hud_color
                                },
                            );

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
                                layer.graphics.add_line(
                                    contact.transform().position
                                        + dir_mat * Vec2::new(data.radii().start, 0.0),
                                    contact.transform().position
                                        + dir_mat * Vec2::new(data.radii().end, 0.0),
                                    hud_thickness,
                                    color,
                                );
                            }

                            // Turret azimuths.
                            if let Some(i) = Self::find_best_armament(
                                contact,
                                false,
                                renderer.to_world_position(context.mouse.position),
                                context.ui.armament,
                            ) {
                                let armament = &data.armaments[i];
                                if armament.entity_type != EntityType::Depositor {
                                    if let Some(turret_index) = armament.turret {
                                        let turret = &data.turrets[turret_index];
                                        let turret_radius = turret
                                            .entity_type
                                            .map(|e| e.data().radius)
                                            .unwrap_or(0.0);
                                        let contact_direction = contact.transform().direction;
                                        let transform = *contact.transform()
                                            + Transform::from_position(turret.position());

                                        let inner: f32 = 0.2 * data.width;
                                        let outer: f32 = 0.325 * data.width;
                                        let arc_thickness: f32 = outer - inner;
                                        let middle: f32 = inner + arc_thickness * 0.5;
                                        let color = hud_color;

                                        // Aim line is only helpful on small turrets.
                                        if turret_radius < inner {
                                            let angle = (contact_direction
                                                + contact.turrets()[turret_index])
                                                .to_radians();
                                            let dir_mat = Mat2::from_angle(angle);
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

                                        let left_back = (contact_direction + turret.angle
                                            - turret.azimuth_bl
                                            + Angle::PI)
                                            .to_radians();
                                        let left_front =
                                            (contact_direction + turret.angle + turret.azimuth_fl)
                                                .to_radians();
                                        let right_back = (contact_direction
                                            + turret.angle
                                            + turret.azimuth_br
                                            + Angle::PI)
                                            .to_radians();
                                        let right_front = (contact_direction + turret.angle
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
                                                    left_back + 2.0 * std::f32::consts::PI
                                                },
                                                arc_thickness,
                                                color,
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Health bar
                        if contact.damage() > Ticks::ZERO {
                            let health_bar_width = 0.12 * zoom;
                            let health_bar_height = 0.0075 * zoom;
                            let health =
                                1.0 - contact.damage().to_secs() / data.max_health().to_secs();
                            let health_back_position = contact.transform().position
                                + Vec2::new(0.0, overlay_vertical_position);
                            let health_bar_position = health_back_position
                                + Vec2::new(
                                    -health_bar_width * 0.5 + health * health_bar_width * 0.5,
                                    0.0,
                                );
                            layer.graphics.add_rectangle(
                                health_back_position,
                                Vec2::new(health_bar_width, health_bar_height),
                                0.0,
                                rgba(85, 85, 85, 127),
                            );
                            layer.graphics.add_rectangle(
                                health_bar_position,
                                Vec2::new(health * health_bar_width, health_bar_height),
                                0.0,
                                color.extend(1.0),
                            );
                        }

                        // Name
                        let text = if let Some(player) =
                            core_state.player_or_bot(contact.player_id().unwrap())
                        {
                            if let Some(team) = player
                                .team_id
                                .and_then(|team_id| core_state.teams.get(&team_id))
                            {
                                format!("[{}] {}", team.team_name, player.alias)
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

                // Amount of particles per frame (scales with FPS).
                let amount =
                    (((data.width * 0.1 + speed * 0.007) * particle_multiplier) as usize).max(1);

                // Wake/thrust particles and shell trails.
                if contact.transform().velocity != Velocity::ZERO
                    && (data.sub_kind != EntitySubKind::Submarine
                        || contact.transform().velocity
                            > Velocity::from_mps(EntityData::CAVITATION_VELOCITY))
                {
                    if data.sub_kind == EntitySubKind::Shell {
                        let t = contact.transform();
                        game_state.trails.add_trail(
                            entity_id,
                            t.position,
                            t.direction.to_vec() * t.velocity.to_mps(),
                            data.width * 2.0,
                        );
                    } else {
                        let layer = if contact.altitude().is_airborne() {
                            &mut layer.airborne_particles
                        } else {
                            &mut layer.sea_level_particles
                        };

                        let spread = match (data.kind, data.sub_kind) {
                            (EntityKind::Weapon, EntitySubKind::Torpedo) => 0.16,
                            _ => 0.1,
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

        game_state.trails.update(&mut layer.airborne_graphics);

        // Play anti-aircraft sfx.
        if anti_aircraft_volume > 0.0 && !layer.audio.is_playing("aa") {
            layer
                .audio
                .play_with_volume("aa", anti_aircraft_volume.min(0.5));
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

        renderer.set_camera(camera, zoom);

        // Send command later, when lifetimes allow.
        let mut control: Option<Command> = None;

        let status = if let Some(player_contact) =
            Self::maybe_contact_mut(&mut game_state.contacts, game_state.entity_id)
        {
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
                }

                if context.mouse.is_down(MouseButton::Right)
                    || context
                        .mouse
                        .is_down_not_click(MouseButton::Left, context.client.update_seconds)
                {
                    let current_dir = player_contact.transform().direction;
                    let mouse_position = renderer.to_world_position(context.mouse.position);
                    let mut direction_target =
                        Angle::from(mouse_position - player_contact.transform().position);

                    // Only do when start holding.
                    if !self.holding {
                        // Don't reverse early on, when the player doesn't have a great idea of their
                        // orientation.
                        if player_contact.entity_type().unwrap().data().level > 1 {
                            // Back 60 degrees turns on reverse.
                            let delta = direction_target - current_dir;
                            self.reversing =
                                delta != delta.clamp_magnitude(Angle::from_degrees(300.0 / 2.0));
                        }
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
                                mouse_position.distance(player_contact.transform().position),
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
            let player_contact = game_state.player_contact().unwrap();

            let status = UiStatus::Alive {
                entity_type: player_contact.entity_type().unwrap(),
                position: player_contact.transform().position.into(),
                direction: player_contact.transform().direction,
                velocity: player_contact.transform().velocity,
                altitude: player_contact.altitude(),
                armament_consumption: Some(player_contact.reloads().into()), // TODO fix to clone arc
            };

            if self.control_rate_limiter.update_ready(elapsed_seconds) {
                let left_click = context.mouse.take_click(MouseButton::Left);

                // Pre-borrow context data for use in closure.
                let mouse_position = renderer.to_world_position(context.mouse.position);

                // Get hint before borrow of player_contact().
                let hint = Some(Hint {
                    aspect: renderer.aspect_ratio(),
                });

                control = Some(Command::Control(Control {
                    guidance: Some(*player_contact.guidance()), // TODO don't send if hasn't changed.
                    angular_velocity_target: None,
                    altitude_target: if player_contact.data().sub_kind == EntitySubKind::Submarine {
                        Some(context.ui.altitude_target)
                    } else {
                        None
                    },
                    aim_target: Some(mouse_position),
                    active: context.ui.active,
                    pay: context.keyboard.is_down(Key::C).then_some(Pay {
                        position: mouse_position,
                    }),
                    fire: if left_click
                        || context
                            .keyboard
                            .state(Key::Space)
                            .combined(context.keyboard.state(Key::E))
                            .is_down()
                    {
                        Self::find_best_armament(
                            player_contact,
                            true,
                            mouse_position,
                            context.ui.armament,
                        )
                        .map(|i| Fire {
                            armament_index: i as u8,
                            position_target: mouse_position,
                        })
                    } else {
                        None
                    },
                    hint,
                }));
            }

            status
        } else {
            UiStatus::Spawning {
                death_reason: game_state.death_reason.as_ref().and_then(|reason| {
                    DeathReasonModel::from_death_reason(reason, &*core_state).ok()
                }),
                connection_lost,
            }
        };

        if let Some(control) = control {
            context.send_to_game(control);
        }

        self.fps_counter.update(elapsed_seconds);

        if self.ui_props_rate_limiter.update_ready(elapsed_seconds) {
            self.update_ui_props(context, status, &team_proximity);
        }
    }

    fn peek_ui(
        &mut self,
        event: &UiEvent,
        context: &mut Context<Self>,
        layer: &mut Self::RendererLayer,
    ) {
        match event {
            UiEvent::Spawn { alias, entity_type } => {
                context.send_to_core(ClientRequest::IdentifySession {
                    alias: PlayerAlias::new(alias),
                });
                context.send_to_game(Command::Spawn(Spawn {
                    entity_type: *entity_type,
                }))
            }
            UiEvent::Upgrade(entity_type) => {
                layer.audio.play("upgrade");
                context.send_to_game(Command::Upgrade(Upgrade {
                    entity_type: *entity_type,
                }))
            }
            UiEvent::Active(active) => {
                if let Some(contact) = context.game().player_contact() {
                    if *active && contact.data().sensors.sonar.range >= 0.0 {
                        layer.audio.play("sonar1")
                    }
                }
            }
            UiEvent::AltitudeTarget(altitude_norm) => {
                let altitude = Altitude::from_norm(*altitude_norm);
                if let Some(contact) = context.game().player_contact() {
                    if contact.data().sub_kind == EntitySubKind::Submarine {
                        if !context.ui.altitude_target.is_submerged() && altitude.is_submerged() {
                            layer.audio.play("dive");
                        } else if context.ui.altitude_target.is_submerged()
                            && !altitude.is_submerged()
                        {
                            layer.audio.play("surface");
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
