// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::animation::Animation;
use crate::interpolated_contact::InterpolatedContact;
use crate::settings::Mk48Settings;
use crate::ui::{
    ChatModel, DeathReasonModel, LeaderboardItemModel, TeamModel, TeamPlayerModel, UiEvent,
    UiProps, UiState, UiStatus,
};
use client_util::apply::Apply;
use client_util::audio::AudioLayer;
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
use client_util::renderer::shader::ShaderBinding;
use client_util::renderer::sprite::SpriteLayer;
use client_util::renderer::text::TextLayer;
use client_util::renderer::texture::Texture;
use client_util::rgb::{gray, rgb, rgba};
use common::altitude::Altitude;
use common::angle::Angle;
use common::contact::{Contact, ContactTrait};
use common::death_reason::DeathReason;
use common::entity::{EntityData, EntityId, EntityKind, EntitySubKind, EntityType};
use common::guidance::Guidance;
use common::protocol::{Command, Control, Fire, Hint, Pay, Spawn, Update, Upgrade};
use common::terrain;
use common::terrain::{Coord, Terrain};
use common::ticks::Ticks;
use common::transform::Transform;
use common::velocity::Velocity;
use common_util::range::{gen_radius, map_ranges};
use core_protocol::id::{GameId, PeriodId, TeamId};
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::ClientRequest;
use glam::{Mat2, Mat3, UVec2, Vec2, Vec4};
use itertools::Itertools;
use rand::{thread_rng, Rng};
use std::cmp::Ordering;
use std::collections::HashMap;

pub struct Mk48Game {
    pub holding: bool,
    pub reversing: bool,
    /// Camera on death.
    pub saved_camera: Option<(Vec2, f32)>,
    /// In meters.
    pub interpolated_zoom: f32,
    /// 1 = normal.
    pub zoom_input: f32,
    pub control_rate_limiter: RateLimiter,
    pub state_rate_limiter: RateLimiter,
    /// Playing the alarm fast sound too often is annoying.
    pub alarm_fast_rate_limiter: RateLimiter,
    /// FPS counter
    pub fps_counter: FpsMonitor,
}

pub struct Mk48BackgroundContext {
    terrain_texture: Option<Texture>,
    sand_texture: Texture,
    grass_texture: Texture,
    matrix: Mat3,
    time: f32,
    visual_range: f32,
    visual_restriction: f32,
    world_radius: f32,
}

impl Mk48BackgroundContext {
    const SAND_COLOR: [u8; 3] = [213, 176, 107];
    const GRASS_COLOR: [u8; 3] = [71, 85, 45];
}

impl BackgroundContext for Mk48BackgroundContext {
    fn prepare(&mut self, shader: &mut ShaderBinding) {
        shader.uniform_texture("uSampler", self.terrain_texture.as_ref().unwrap(), 0);
        // TODO don't bind textures if not enabled.
        shader.uniform_texture("uSand", &self.sand_texture, 1);
        shader.uniform_texture("uGrass", &self.grass_texture, 2);
        shader.uniform_matrix3f("uTexture", &self.matrix);
        shader.uniform4f(
            "uTime_uVisual_uRestrict_uBorder",
            Vec4::new(
                self.time,
                self.visual_range,
                self.visual_restriction,
                self.world_radius,
            ),
        );
    }
}

/// State associated with game server connection. Reset when connection is reset.
#[derive(Default)]
pub struct Mk48State {
    pub score: u32,
    pub entity_id: Option<EntityId>,
    pub contacts: HashMap<EntityId, InterpolatedContact>,
    pub animations: Vec<Animation>,
    pub terrain: Terrain,
    pub world_radius: f32,
    pub death_reason: Option<DeathReason>,
}

impl Mk48State {
    /// Returns the "view" of the player's boat's contact, if the player has a boat.
    pub(crate) fn player_contact(&self) -> Option<&Contact> {
        self.entity_id
            .map(|id| &self.contacts.get(&id).unwrap().view)
    }
}

impl Apply<Update> for Mk48State {
    fn apply(&mut self, update: Update) {
        self.death_reason = update.death_reason;
        self.terrain.apply_update(&update.terrain);
        self.world_radius = update.world_radius;
        self.score = update.score;
    }
}

/// Order of fields is order of rendering.
#[derive(Layer)]
pub struct RendererLayer {
    audio: AudioLayer,
    background: BackgroundLayer<Mk48BackgroundContext>,
    sea_level_particles: ParticleLayer,
    sprites: SpriteLayer,
    airborne_particles: ParticleLayer,
    graphics: GraphicLayer,
    text: TextLayer,
}

impl Mk48Game {
    pub(crate) fn volume_at(distance: f32) -> f32 {
        1.0 / (1.0 + 0.05 * distance)
    }

    fn play_music(name: &'static str, audio_player: &AudioLayer) {
        // Highest to lowest.
        let music_priorities = ["achievement", "dodge", "intense"];

        let index = music_priorities
            .iter()
            .position(|&m| m == name)
            .expect("name must be one of available music");

        for (i, music) in music_priorities.iter().enumerate() {
            if audio_player.is_playing(music) {
                if i <= index {
                    // Preempted by higher priority music, or already playing.
                    return;
                } else {
                    // Preempt lower priority music.
                    audio_player.stop_playing(music);
                }
            }
        }

        audio_player.play(name);
    }

    /// Finds the best armament (i.e. the one that will be fired if the mouse is clicked).
    /// Armaments are scored by a combination of distance and angle to target.
    fn find_best_armament(
        &self,
        player_contact: &Contact,
        angle_limit: bool,
        mouse_position: Vec2,
        armament_selection: Option<(EntityKind, EntitySubKind)>,
    ) -> Option<usize> {
        // The f32 represents how good the shot is, lower is better.
        let mut best_armament: Option<(usize, f32)> = None;

        if let Some(armament_selection) = armament_selection {
            for i in 0..player_contact.data().armaments.len() {
                let armament = &player_contact.data().armaments[i];

                let armament_entity_data: &EntityData = armament.entity_type.data();

                if !(armament_entity_data.kind == armament_selection.0
                    && armament_entity_data.sub_kind == armament_selection.1)
                {
                    // Wrong type; cannot fire.
                    continue;
                }

                if player_contact.reloads()[i] != Ticks::ZERO {
                    // Reloading; cannot fire.
                    continue;
                }

                if let Some(turret_index) = armament.turret {
                    if !player_contact.data().turrets[turret_index]
                        .within_azimuth(player_contact.turrets()[turret_index])
                    {
                        // Out of azimuth range; cannot fire.
                        continue;
                    }
                }

                let transform = *player_contact.transform()
                    + player_contact
                        .data()
                        .armament_transform(player_contact.turrets(), i);

                let armament_direction_target = Angle::from(mouse_position - transform.position);

                let mut angle_diff = (armament_direction_target - transform.direction).abs();
                let distance_squared = mouse_position.distance_squared(transform.position);
                if armament.vertical
                    || armament_entity_data.kind == EntityKind::Aircraft
                    || armament_entity_data.sub_kind == EntitySubKind::Depositor
                    || armament_entity_data.sub_kind == EntitySubKind::DepthCharge
                    || armament_entity_data.sub_kind == EntitySubKind::Mine
                {
                    // Vertically-launched armaments can fire in any horizontal direction.
                    // Aircraft can quickly assume any direction.
                    // Depositors, depth charges, and mines are not constrained by direction.
                    angle_diff = Angle::ZERO;
                }

                let max_angle_diff = match armament_entity_data.sub_kind {
                    EntitySubKind::Shell => Angle::from_degrees(30.0),
                    EntitySubKind::Rocket => Angle::from_degrees(45.0),
                    EntitySubKind::Torpedo if armament_entity_data.sensors.sonar.range > 0.0 => {
                        Angle::from_degrees(150.0)
                    }
                    _ => Angle::from_degrees(90.0),
                };

                if !angle_limit || angle_diff < max_angle_diff {
                    let score = angle_diff.to_degrees().powi(2) + distance_squared;
                    if best_armament.map(|(_, s)| score < s).unwrap_or(true) {
                        best_armament = Some((i, score));
                    }
                }
            }
        }

        best_armament.map(|(idx, _)| idx)
    }
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
            interpolated_zoom: 10.0,
            zoom_input: 0.5,
            saved_camera: None,
            control_rate_limiter: RateLimiter::new(0.1),
            state_rate_limiter: RateLimiter::new(0.1),
            alarm_fast_rate_limiter: RateLimiter::new(10.0), // TODO: is this an aspect of rendering or of the game?
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

        let audio_sprite_sheet =
            serde_json::from_str(include_str!("./sprites_audio.json")).unwrap();
        let sprite_sheet = serde_json::from_str(include_str!("./sprites_webgl.json")).unwrap();
        let sprite_texture =
            renderer.load_texture("/sprites_webgl.png", UVec2::new(2048, 2048), None, false);
        let wind = Vec2::new(7.0, 1.5);

        let (sand_path, grass_path) = if context.settings.render_terrain_textures {
            ("/sand.png", "/grass.png")
        } else {
            // TODO: Kludge, don't load textures.
            ("/dummy.png", "/dummy.png")
        };

        let background_context = Mk48BackgroundContext {
            terrain_texture: None,
            sand_texture: renderer.load_texture(
                sand_path,
                UVec2::new(256, 256),
                Some(Mk48BackgroundContext::SAND_COLOR),
                true,
            ),
            grass_texture: renderer.load_texture(
                grass_path,
                UVec2::new(256, 256),
                Some(Mk48BackgroundContext::GRASS_COLOR),
                true,
            ),
            matrix: Mat3::IDENTITY,
            time: 0.0,
            visual_range: 0.0,
            visual_restriction: 0.0,
            world_radius: 0.0,
        };

        RendererLayer {
            audio: AudioLayer::new("/sprites_audio.mp3", audio_sprite_sheet),
            background: BackgroundLayer::new(renderer, background_shader, background_context),
            sea_level_particles: ParticleLayer::new(renderer, Vec2::ZERO),
            sprites: SpriteLayer::new(renderer, sprite_texture, sprite_sheet),
            airborne_particles: ParticleLayer::new(renderer, wind),
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
        layer: &Self::RendererLayer,
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
                Self::propagate_contact(&mut network_contact.model, 0.1);
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
            self.lost_contact(
                renderer.camera_center(),
                &contact,
                &layer.audio,
                &mut context.game_mut().animations,
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

                let friendly = &context.core().is_friendly(contact.player_id());
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

        let debug_latency_player_entity_id = if false { game_state.entity_id } else { None };
        // A subset of game logic.
        for InterpolatedContact {
            model, view, error, ..
        } in &mut game_state.contacts.values_mut()
        {
            if model
                .entity_type()
                .map(|e| e.data().kind == EntityKind::Boat)
                .unwrap_or(false)
            {
                // Update team_proximity.
                if let Some(player_id) = model.player_id() {
                    if let Some(player) = core_state.players.get(&player_id) {
                        if let Some(team_id) = player.team_id {
                            let distance = camera.distance_squared(model.transform().position);
                            team_proximity
                                .entry(team_id)
                                .and_modify(|dist| *dist = dist.min(distance))
                                .or_insert(distance);
                        }
                    }
                }
            }

            let positional_inaccuracy = model
                .transform()
                .position
                .distance_squared(view.transform().position);
            let directional_inaccuracy = (model.transform().direction - view.transform().direction)
                .abs()
                .to_radians();
            let velocity_inaccuracy = model
                .transform()
                .velocity
                .difference(view.transform().velocity)
                .to_mps();

            if Some(view.id()) == debug_latency_player_entity_id {
                client_util::console_log!(
                    "err: {:.2}, pos: {:.2}, dir: {:.2}, vel: {:.2}",
                    *error,
                    positional_inaccuracy.sqrt(),
                    directional_inaccuracy,
                    velocity_inaccuracy
                );
            }
            *error = (*error * 0.5f32.powf(elapsed_seconds)
                + elapsed_seconds
                    * (positional_inaccuracy * 0.4
                        + directional_inaccuracy * 2.0
                        + velocity_inaccuracy * 0.08))
                .clamp(0.0, 10.0);

            // If reloads are known before and after, and one goes from zero to non-zero, it was fired.
            if let Some(entity_type) = model.entity_type() {
                let data: &EntityData = entity_type.data();
                if view.entity_type() == model.entity_type()
                    && view.reloads_known()
                    && model.reloads_known()
                    && view.turrets_known()
                {
                    let model_reloads = model.reloads();
                    for (i, &old) in view.reloads().iter().enumerate() {
                        let new = model_reloads[i];

                        if new == Ticks::ZERO || old != Ticks::ZERO {
                            // Wasn't just fired
                            continue;
                        }

                        let armament = &data.armaments[i];
                        let armament_entity_data = armament.entity_type.data();

                        if !matches!(
                            armament_entity_data.sub_kind,
                            EntitySubKind::Shell | EntitySubKind::Rocket | EntitySubKind::Missile
                        ) {
                            // Don't generate particles.
                            continue;
                        }

                        let boat_velocity = view.transform().direction.to_vec()
                            * view.transform().velocity.to_mps();

                        let armament_transform =
                            *view.transform() + data.armament_transform(view.turrets(), i);

                        let direction_vector: Vec2 = if armament.vertical {
                            // Straight up.
                            Vec2::ZERO
                        } else {
                            armament_transform.direction.into()
                        };

                        let mut rng = thread_rng();

                        let forward_offset = armament
                            .turret
                            .and_then(|t| data.turrets[t].entity_type)
                            .map(|t| t.data().length * 0.4)
                            .unwrap_or(2.0);
                        let forward_velocity = 0.5 * armament_entity_data.speed.to_mps().min(100.0);

                        let layer = if view.altitude().is_submerged() {
                            &mut layer.sea_level_particles
                        } else {
                            &mut layer.airborne_particles
                        };

                        // Add muzzle flash particles.
                        let amount = 10;
                        for i in 0..amount {
                            layer.add(Particle {
                                position: armament_transform.position
                                    + direction_vector * forward_offset,
                                velocity: boat_velocity
                                    + direction_vector
                                        * forward_velocity
                                        * (i as f32 * (1.0 / amount as f32))
                                    + direction_vector.perp()
                                        * forward_velocity
                                        * 0.15
                                        * (rng.gen::<f32>() - 0.5),
                                radius: (armament_entity_data.width * 5.0).clamp(1.0, 3.0),
                                color: -1.0,
                            });
                        }
                    }
                }
            }

            // Don't interpolate view's guidance if this is the player's boat, so that it doesn't jerk around.
            view.interpolate_towards(
                model,
                Some(model.id()) != game_state.entity_id,
                elapsed_seconds * (*error),
                elapsed_seconds,
            );
            for contact in [model, view] {
                Self::propagate_contact(contact, elapsed_seconds);
            }
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

        // Both width and height must be odd numbers so there is an equal distance from the center
        // on both sides.
        let terrain_width: usize = 2 * ((zoom / terrain::SCALE).max(2.0) as usize + 1) + 3;
        let terrain_height =
            2 * ((zoom / (renderer.aspect_ratio() * terrain::SCALE)).max(2.0) as usize + 1) + 3;

        let mut terrain_bytes = Vec::with_capacity(terrain_width * terrain_height);
        let terrain_center = Coord::from_position(camera).unwrap();

        terrain_bytes.extend(game_state.terrain.iter_rect_or(
            terrain_center,
            terrain_width,
            terrain_height,
            0,
        ));

        let terrain_offset = Mat3::from_translation(-terrain_center.corner());
        let terrain_scale = &Mat3::from_scale(Vec2::new(
            1.0 / (terrain_width as f32 * terrain::SCALE),
            1.0 / (terrain_height as f32 * terrain::SCALE),
        ));

        // This matrix converts from world space to terrain texture UV coordinates.
        layer.background.context.matrix = Mat3::from_translation(Vec2::new(0.5, 0.5))
            .mul_mat3(&terrain_scale.mul_mat3(&terrain_offset));

        renderer.realloc_texture_from_bytes(
            &mut layer.background.context.terrain_texture,
            terrain_width as u32,
            terrain_height as u32,
            &terrain_bytes,
        );

        layer.background.context.time = context.client.update_seconds;
        layer.background.context.visual_range = visual_range;
        layer.background.context.visual_restriction = visual_restriction;
        layer.background.context.world_radius = game_state.world_radius;

        struct SortableSprite<'a> {
            alpha: f32,
            altitude: f32,
            dimensions: Vec2,
            entity_id: Option<EntityId>,
            frame: Option<usize>,
            sprite: &'a str,
            transform: Transform,
        }

        impl<'a> SortableSprite<'a> {
            fn new_entity(
                entity_id: EntityId,
                entity_type: EntityType,
                transform: Transform,
                mut altitude: f32,
                alpha: f32,
            ) -> Self {
                altitude += Self::entity_height(entity_type);
                Self {
                    sprite: entity_type.as_str(),
                    frame: None,
                    dimensions: entity_type.data().dimensions(),
                    transform,
                    altitude,
                    alpha,
                    entity_id: Some(entity_id),
                }
            }

            fn new_child_entity(
                entity_id: EntityId,
                parent_type: EntityType,
                entity_type: EntityType,
                transform: Transform,
                mut altitude: f32,
                alpha: f32,
            ) -> Self {
                altitude += Self::entity_height(parent_type);
                Self::new_entity(entity_id, entity_type, transform, altitude, alpha)
            }

            fn new_animation(animation: &Animation) -> Self {
                Self {
                    alpha: 1.0,
                    altitude: animation.altitude,
                    dimensions: Vec2::splat(animation.scale),
                    entity_id: None,
                    frame: Some(animation.frame),
                    sprite: animation.name,
                    transform: Transform::from_position(animation.position),
                }
            }

            fn entity_height(entity_type: EntityType) -> f32 {
                entity_type.data().length * 0.0001
            }
        }

        impl<'a> PartialEq for SortableSprite<'a> {
            fn eq(&self, other: &Self) -> bool {
                self.altitude == other.altitude && self.entity_id == other.entity_id
            }
        }

        impl<'a> PartialOrd for SortableSprite<'a> {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(
                    self.altitude
                        .partial_cmp(&other.altitude)?
                        .then_with(|| self.entity_id.cmp(&other.entity_id)),
                )
            }
        }

        // Prepare to sort sprites.
        let mut sortable_sprites = Vec::with_capacity(game_state.contacts.len() * 5);

        // Update animations.
        let mut i = 0;
        while i < game_state.animations.len() {
            let animation = &mut game_state.animations[i];
            animation.update(elapsed_seconds);

            let len = layer.sprites.animation_length(animation.name);

            if animation.frame >= len {
                game_state.animations.swap_remove(i);
            } else {
                sortable_sprites.push(SortableSprite::new_animation(animation));
                i += 1;
            }
        }

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
                            if let Some(i) = self.find_best_armament(
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
                        let text = if let Some(player) = core_state
                            .players
                            .get(contact.player_id().as_ref().unwrap())
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

                // Wake/trail particles.
                if contact.transform().velocity != Velocity::ZERO
                    && (data.sub_kind != EntitySubKind::Submarine
                        || contact.transform().velocity
                            > Velocity::from_mps(EntityData::CAVITATION_VELOCITY))
                {
                    let layer = if contact.altitude().is_airborne() {
                        &mut layer.airborne_particles
                    } else {
                        &mut layer.sea_level_particles
                    };

                    let spread = match (data.kind, data.sub_kind) {
                        (EntityKind::Weapon, EntitySubKind::Torpedo) => 0.16,
                        (EntityKind::Weapon, EntitySubKind::Shell) => 0.0,
                        _ => 0.1,
                    };

                    let start = contact.transform().position;
                    let end = start + direction_vector * speed * elapsed_seconds;

                    let factor = 1.0 / amount as f32;
                    for i in 0..amount {
                        let pos = start.lerp(end, i as f32 * factor);

                        let r = rng.gen::<f32>() - 0.5;
                        layer.add(Particle {
                            position: pos - direction_vector * (data.length * 0.485)
                                + tangent_vector * (data.width * r * 0.25),
                            velocity: direction_vector * (speed * 0.75)
                                + tangent_vector * (speed * r * spread),
                            radius: 1.0,
                            color: 1.0,
                        });
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
                        self.find_best_armament(
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
                death_reason: game_state.death_reason.as_ref().map(|reason| match reason {
                    DeathReason::Border => DeathReasonModel {
                        death_type: "border",
                        player: None,
                        entity: None,
                    },
                    DeathReason::Terrain => DeathReasonModel {
                        death_type: "terrain",
                        player: None,
                        entity: None,
                    },
                    DeathReason::Boat(player_id) => DeathReasonModel {
                        death_type: "collision",
                        player: Some(
                            core_state
                                .players
                                .get(player_id)
                                .map(|p| p.alias)
                                .unwrap_or_else(|| PlayerAlias::new("???")),
                        ),
                        entity: None,
                    },
                    DeathReason::Entity(entity_type) => DeathReasonModel {
                        death_type: "collision",
                        player: None,
                        entity: Some(*entity_type),
                    },
                    DeathReason::Ram(player_id) => DeathReasonModel {
                        death_type: "ramming",
                        player: Some(
                            core_state
                                .players
                                .get(player_id)
                                .map(|p| p.alias)
                                .unwrap_or_else(|| PlayerAlias::new("???")),
                        ),
                        entity: None,
                    },
                    DeathReason::Weapon(player_id, entity_type) => DeathReasonModel {
                        death_type: "sinking",
                        player: Some(
                            core_state
                                .players
                                .get(player_id)
                                .map(|p| p.alias)
                                .unwrap_or_else(|| PlayerAlias::new("???")),
                        ),
                        entity: Some(*entity_type),
                    },
                    _ => panic!("invalid death reason for boat: {:?}", reason),
                }),
                connection_lost,
            }
        };

        self.fps_counter.update(elapsed_seconds);

        if self.state_rate_limiter.update_ready(elapsed_seconds) {
            let props = UiProps {
                player_id: core_state.player_id,
                team_name: core_state.team().map(|t| t.team_name),
                invitation_id: core_state.created_invitation_id,
                score: game_state.score,
                player_count: core_state.player_count,
                fps: self.fps_counter.last_sample().unwrap_or(0.0),
                status,
                chats: core_state
                    .messages
                    .iter()
                    .map(|message| ChatModel {
                        name: message.alias,
                        player_id: message.player_id,
                        team: message.team_name,
                        message: message.text.clone(),
                        whisper: message.whisper,
                    })
                    .collect(),
                liveboard: context
                    .core()
                    .liveboard
                    .iter()
                    .filter_map(|item| {
                        let player = core_state.players.get(&item.player_id);
                        if let Some(player) = player {
                            let team_name = player
                                .team_id
                                .and_then(|team_id| core_state.teams.get(&team_id))
                                .map(|team| team.team_name);
                            Some(LeaderboardItemModel {
                                name: player.alias,
                                team: team_name,
                                score: item.score,
                            })
                        } else {
                            None
                        }
                    })
                    .collect(),
                leaderboards: context
                    .core()
                    .leaderboards
                    .iter()
                    .enumerate()
                    .map(|(i, leaderboard)| {
                        let period: PeriodId = i.into();
                        (
                            period,
                            leaderboard
                                .iter()
                                .map(|item| LeaderboardItemModel {
                                    name: item.alias,
                                    team: None,
                                    score: item.score,
                                })
                                .collect(),
                        )
                    })
                    .collect(),
                team_members: if let Some(team_id) = core_state.team_id() {
                    core_state
                        .players
                        .values()
                        .filter(|p| p.team_id == Some(team_id))
                        .map(|p| TeamPlayerModel {
                            player_id: p.player_id,
                            name: p.alias,
                            captain: p.team_captain,
                        })
                        .sorted_by(|a, b| b.captain.cmp(&a.captain).then(a.name.cmp(&b.name)))
                        .collect()
                } else {
                    vec![]
                },
                team_captain: core_state.team_id().is_some()
                    && core_state.player().map(|p| p.team_captain).unwrap_or(false),
                team_join_requests: context
                    .core()
                    .joiners
                    .iter()
                    .filter_map(|id| {
                        context
                            .core()
                            .players
                            .get(id)
                            .map(|player| TeamPlayerModel {
                                player_id: player.player_id,
                                name: player.alias,
                                captain: false,
                            })
                    })
                    .collect(),
                teams: context
                    .core()
                    .teams
                    .iter()
                    .sorted_by(|&(a, _), &(b, _)| {
                        team_proximity
                            .get(a)
                            .unwrap_or(&f32::INFINITY)
                            .partial_cmp(team_proximity.get(b).unwrap_or(&f32::INFINITY))
                            .unwrap()
                    })
                    .map(|(team_id, team)| TeamModel {
                        team_id: *team_id,
                        name: team.team_name,
                        joining: core_state.joins.contains(team_id),
                    })
                    .take(5)
                    .collect(),
            };

            context.set_ui_props(props);
        }

        if let Some(control) = control {
            context.send_to_game(control);
        }
    }

    fn peek_ui(
        &mut self,
        event: &UiEvent,
        context: &mut Context<Self>,
        layer: &mut Self::RendererLayer,
    ) {
        match *event {
            UiEvent::Spawn { alias, entity_type } => {
                context.send_to_core(ClientRequest::IdentifySession { alias });
                context.send_to_game(Command::Spawn(Spawn { entity_type }))
            }
            UiEvent::Upgrade(entity_type) => {
                layer.audio.play("upgrade");
                context.send_to_game(Command::Upgrade(Upgrade { entity_type }))
            }
            UiEvent::Active(active) => {
                if let Some(contact) = context.game().player_contact() {
                    if active && contact.data().sensors.sonar.range >= 0.0 {
                        layer.audio.play("sonar1")
                    }
                }
            }
            UiEvent::AltitudeTarget(altitude_norm) => {
                let altitude = Altitude::from_norm(altitude_norm);
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
