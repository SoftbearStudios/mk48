// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::animation::Animation;
use crate::audio::AudioPlayer;
use crate::input::Input;
use crate::particle::Particle;
use crate::reconn_web_socket::ReconnWebSocket;
use crate::renderer::Renderer;
use crate::settings::Settings;
use crate::text_cache::TextCache;
use crate::texture::Texture;
use crate::util::{domain_name, gray, host, referrer, rgb, rgba, ws_protocol, FpsMonitor};
use crate::{
    has_webp, ChatModel, DeathReasonModel, LeaderboardItemModel, State, Status, TeamModel,
    TeamPlayerModel,
};
use common::altitude::Altitude;
use common::angle::Angle;
use common::contact::{Contact, ContactTrait};
use common::death_reason::DeathReason;
use common::entity::*;
use common::guidance::Guidance;
use common::protocol::*;
use common::terrain::{Coord, Terrain};
use common::ticks::Ticks;
use common::transform::Transform;
use common::util::{gen_radius, map_ranges};
use common::velocity::Velocity;
use common::{terrain, util};
use core_protocol::dto::*;
use core_protocol::id::*;
use core_protocol::name::*;
use core_protocol::rpc::*;
use core_protocol::web_socket::WebSocketFormat;
use glam::{vec2, Mat2, Mat3, Vec2};
use itertools::Itertools;
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet, VecDeque};

pub struct Game {
    contacts: HashMap<EntityId, NetworkContact>,
    chats: VecDeque<MessageDto>,
    players: HashMap<PlayerId, PlayerDto>,
    joiners: HashSet<PlayerId>,
    joins: HashSet<TeamId>,
    teams: HashMap<TeamId, TeamDto>,
    leaderboards: HashMap<PeriodId, Vec<LeaderboardDto>>,
    liveboard: Vec<LiveboardDto>,
    /// Created invitation
    created_invitation_id: Option<InvitationId>,
    sea_level_particles: Vec<Particle>,
    airborne_particles: Vec<Particle>,
    animations: Vec<Animation>,
    terrain: Terrain,
    player_count: u32,
    world_radius: f32,
    /// Camera from alive boat, useful to maintain view after death.
    saved_camera: Option<(Vec2, f32)>,
    arena_id: Option<ArenaId>,
    session_id: Option<SessionId>,
    death_reason: Option<DeathReason>,
    last_time_seconds: f32,
    /// Time last sent Control command.
    last_control_seconds: f32,
    player_id: Option<PlayerId>,
    entity_id: Option<EntityId>,
    pub input: Input,
    // Actual zoom ratio (smoothed over time).
    zoom: f32,
    renderer: Renderer,
    pub audio_player: AudioPlayer,
    score: u32,
    pub server_web_socket: Option<ReconnWebSocket<Update, Command>>,
    pub(crate) core_web_socket: ReconnWebSocket<ClientUpdate, ClientRequest>,
    terrain_texture: Option<Texture>,
    text_cache: TextCache,
    fps_monitor: FpsMonitor,
}

/// A contact that may be locally controlled by simulated elsewhere (by the server).
struct NetworkContact {
    /// The more accurate representation of the contact, which is snapped to server updates.
    model: Contact,
    /// The visual representation of the contact, which is gradually interpolated towards model.
    view: Contact,
    /// Integrate error to control rubber banding strength. Having an error for longer means stronger
    /// interpolation back to model.
    error: f32,
    /// Idle ticks, i.e. how many updates since last seen. If exceeds entity_type.data().keey_alive(),
    /// assume entity went away.
    idle: Ticks,
}

impl NetworkContact {
    fn new(contact: Contact) -> Self {
        // When a new contact appears, its model and view are identical.
        Self {
            model: contact.clone(),
            view: contact,
            error: 0.0,
            idle: Ticks::ZERO,
        }
    }
}

impl Game {
    /// Time, in seconds, between sending Control commands.
    const CONTROL_PERIOD: f32 = 0.1;

    pub fn new(
        settings: Settings,
        saved_session_tuple: Option<(ArenaId, SessionId)>,
        invitation_id: Option<InvitationId>,
    ) -> Self {
        let sprite_path = if has_webp() {
            "/sprites_webgl.webp"
        } else {
            "/sprites_webgl.png"
        };
        let sprite_sheet = serde_json::from_str(include_str!("./sprites_webgl.json")).unwrap();

        let renderer = Renderer::new(settings, sprite_path, sprite_sheet);

        let audio_sprite_sheet =
            serde_json::from_str(include_str!("./sprites_audio.json")).unwrap();
        let audio_player = AudioPlayer::new("/sprites_audio.mp3", audio_sprite_sheet);

        let host = if let Some(invitation_id) = invitation_id {
            if let Some(server_id) = invitation_id.server_id() {
                format!("{}.{}", server_id.0, domain_name())
            } else {
                host()
            }
        } else {
            host()
        };

        let core_web_socket = ReconnWebSocket::new(
            &format!("{}://{}/client/ws/", ws_protocol(), host),
            WebSocketFormat::Json,
            Some(ClientRequest::CreateSession {
                game_id: GameId::Mk48,
                invitation_id,
                referrer: referrer(),
                saved_session_tuple,
            }),
        );

        Self {
            input: Input::new(),
            zoom: 10.0,
            last_time_seconds: 0.0,
            last_control_seconds: 0.0,
            renderer,
            audio_player,
            contacts: HashMap::new(),
            chats: Vec::new().into(),
            players: HashMap::new(),
            joiners: HashSet::new(),
            joins: HashSet::new(),
            leaderboards: HashMap::new(),
            liveboard: Vec::new(),
            teams: HashMap::new(),
            sea_level_particles: Vec::new(),
            airborne_particles: Vec::new(),
            animations: Vec::new(),
            terrain: Terrain::new(),
            player_id: None,
            entity_id: None,
            arena_id: None,
            session_id: None,
            death_reason: None,
            score: 0,
            player_count: 0,
            server_web_socket: None,
            core_web_socket,
            terrain_texture: None,
            text_cache: TextCache::new(),
            world_radius: 10000.0,
            saved_camera: None,
            created_invitation_id: None,
            fps_monitor: FpsMonitor::new(),
        }
    }

    fn volume_at(&self, point: Vec2) -> f32 {
        let distance = self.camera().0.distance(point);
        1.0 / (1.0 + 0.05 * distance)
    }

    fn play_music(&self, name: &'static str) {
        // Highest to lowest.
        let music_priorities = ["achievement", "dodge", "intense"];

        let index = music_priorities
            .iter()
            .position(|&m| m == name)
            .expect("name must be one of available music");

        for (i, music) in music_priorities.iter().enumerate() {
            if self.audio_player.is_playing(music) {
                if i <= index {
                    // Preempted by higher priority music, or already playing.
                    return;
                } else {
                    // Preempt lower priority music.
                    self.audio_player.stop_playing(music);
                }
            }
        }

        self.audio_player.play(name);
    }

    pub fn lost_contact(&mut self, contact: &Contact) {
        if let Some(entity_type) = contact.entity_type() {
            // Contact lost (of a previously known entity type), spawn a splash and make a sound.
            let volume = self.volume_at(contact.transform().position).min(0.25);
            let name = match entity_type.data().kind {
                EntityKind::Boat | EntityKind::Aircraft => "splash",
                EntityKind::Weapon => match entity_type.data().sub_kind {
                    EntitySubKind::Missile
                    | EntitySubKind::Sam
                    | EntitySubKind::Rocket
                    | EntitySubKind::Shell => "explosion",
                    _ => "splash",
                },
                EntityKind::Collectible => {
                    self.audio_player.play_with_volume("collect", volume);
                    return;
                }
                _ => return,
            };

            if entity_type.data().kind == EntityKind::Boat {
                self.audio_player.play_with_volume("explosion_long", volume);
            } else {
                self.audio_player
                    .play_with_volume("explosion_short", volume);
            }

            self.animations.push(Animation::new(
                name,
                contact.transform().position,
                contact.altitude().to_norm(),
                10.0,
            ));
        }
    }

    pub fn new_contact(&self, contact: &Contact) {
        let player_position = self.camera().0;
        let position_diff = contact.transform().position - player_position;
        let direction = Angle::from(position_diff);
        let inbound = (contact.transform().direction - direction + Angle::PI).abs() < Angle::PI_2;

        let friendly = Self::is_friendly(self.player_id, contact, &self.players);
        let volume = self.volume_at(contact.transform().position);

        if let Some(entity_type) = contact.entity_type() {
            let data: &EntityData = entity_type.data();

            match data.kind {
                EntityKind::Boat => {
                    if !friendly && inbound && self.entity_id.is_some() {
                        self.audio_player
                            .play_with_volume("alarm_slow", 0.25 * volume.max(0.5));
                    }
                }
                EntityKind::Weapon => match data.sub_kind {
                    EntitySubKind::Torpedo => {
                        if friendly {
                            self.audio_player
                                .play_with_volume("torpedo_launch", volume.min(0.5));
                            self.audio_player
                                .play_with_volume_and_delay("splash", volume, 0.1);
                        }
                        if data.sensors.sonar.range > 0.0 {
                            self.audio_player.play_with_volume_and_delay(
                                "sonar3",
                                volume,
                                if friendly { 1.0 } else { 0.0 },
                            );
                        }
                    }
                    EntitySubKind::Missile | EntitySubKind::Rocket => {
                        if !friendly && inbound && self.entity_id.is_some() {
                            self.audio_player
                                .play_with_volume("alarm_fast", volume.max(0.5));
                        }
                        self.audio_player.play_with_volume("rocket", volume);
                    }
                    EntitySubKind::Sam => {
                        self.audio_player.play_with_volume("rocket", volume);
                    }
                    EntitySubKind::DepthCharge | EntitySubKind::Mine => {
                        self.audio_player.play_with_volume("splash", volume);
                        if !friendly && self.entity_id.is_some() {
                            self.audio_player
                                .play_with_volume("alarm_slow", volume.max(0.5));
                        }
                    }
                    EntitySubKind::Shell => {
                        self.audio_player.play_with_volume(
                            "shell",
                            volume * map_ranges(data.length, 0.5..1.5, 0.5..1.0, true),
                        );
                    }
                    _ => {}
                },
                EntityKind::Aircraft => {
                    if !friendly && inbound {
                        self.audio_player
                            .play_with_volume("alarm_slow", 0.1 * volume.max(0.5));
                    }
                }
                EntityKind::Decoy => match data.sub_kind {
                    EntitySubKind::Sonar => self.audio_player.play_with_volume("sonar3", volume),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn team_id_lookup(
        player_id: PlayerId,
        players: &HashMap<PlayerId, PlayerDto>,
    ) -> Option<TeamId> {
        players.get(&player_id).and_then(|p| p.team_id)
    }

    /// Either player_id's must equal or team_id's must equal to be considered friendly.
    fn is_friendly(
        player_id: Option<PlayerId>,
        contact: &Contact,
        players: &HashMap<PlayerId, PlayerDto>,
    ) -> bool {
        player_id
            .zip(contact.player_id())
            .map(|(id1, id2)| {
                id1 == id2
                    || Self::team_id_lookup(id1, players)
                        .zip(Self::team_id_lookup(id2, players))
                        .map(|(id1, id2)| id1 == id2)
                        .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    pub fn update(&mut self, update: Update) {
        self.player_id = Some(update.player_id);

        let updated: HashMap<EntityId, Contact> =
            update.contacts.into_iter().map(|c| (c.id(), c)).collect();

        for (id, contact) in updated.iter() {
            if let Some(NetworkContact { model, .. }) = self.contacts.get(&id) {
                if Some(*id) == self.entity_id {
                    let recent_damage = contact.damage().saturating_sub(model.damage());
                    if recent_damage > Ticks::ZERO {
                        self.audio_player.play("damage");

                        // Considered "intense" 250% of the damage would have been fatal.
                        if recent_damage * 2.5
                            >= model.data().max_health().saturating_sub(model.damage())
                        {
                            self.play_music("intense");
                        }
                    }
                }

                // Mutable borrow after immutable borrows.
                let network_contact = self.contacts.get_mut(&id).unwrap();
                network_contact.model = contact.clone();
            } else {
                self.new_contact(&contact);
                if contact.player_id() == self.player_id && contact.is_boat() {
                    self.entity_id = Some(contact.id());
                }
                self.contacts
                    .insert(contact.id(), NetworkContact::new(contact.clone()));
            }
        }

        // Contacts absent in the update are currently considered lost.
        // Borrow entity_id early to avoid use of self in closure.
        let entity_id = &mut self.entity_id;
        for contact in self
            .contacts
            .drain_filter(|id, NetworkContact { idle, view, .. }| {
                if updated.contains_key(&id) {
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
            .map(|(_, NetworkContact { view, .. })| view)
            .collect::<Vec<_>>()
        {
            self.lost_contact(&contact);
        }

        let player_position = self.camera().0;
        let player_altitude = self
            .player_contact()
            .map(|c| c.altitude())
            .unwrap_or(Altitude::ZERO);
        let mut aircraft_volume: f32 = 0.0;
        let mut need_to_dodge: f32 = 0.0;

        for (_, NetworkContact { view: contact, .. }) in self.contacts.iter() {
            if let Some(entity_type) = contact.entity_type() {
                let data: &'static EntityData = entity_type.data();
                let position_diff = contact.transform().position - player_position;
                let direction = Angle::from(position_diff);
                let distance = position_diff.length();
                let inbound =
                    (contact.transform().direction - direction + Angle::PI).abs() < Angle::PI_2;

                let friendly = Self::is_friendly(self.player_id, contact, &self.players);
                let volume = self.volume_at(contact.transform().position);

                if data.kind == EntityKind::Aircraft {
                    aircraft_volume += volume;
                }

                if self.entity_id.is_some() && distance < 250.0 {
                    let distance_scale = 1000.0 / (500.0 + distance);
                    match data.kind {
                        EntityKind::Boat => {
                            if !friendly
                                && inbound
                                && self.entity_id.is_some()
                                && data.sub_kind == EntitySubKind::Ram
                                && !player_altitude.is_submerged()
                            {
                                need_to_dodge += 2.0 * distance_scale;
                            }
                        }
                        EntityKind::Weapon => match data.sub_kind {
                            EntitySubKind::Torpedo => {
                                if inbound && !friendly && self.entity_id.is_some() {
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
            self.audio_player
                .play_with_volume("aircraft", (aircraft_volume + 1.0).ln());
        }

        if need_to_dodge >= 3.0 {
            self.play_music("dodge");
        }

        let score_delta = update.score.saturating_sub(self.score);
        if score_delta >= 10 && (score_delta >= 200 || score_delta as f32 / self.score as f32 > 0.5)
        {
            self.play_music("achievement");
        }
        self.score = update.score;

        self.death_reason = update.death_reason;
        self.world_radius = update.world_radius;
        self.terrain.apply_update(&update.terrain);
    }

    fn player_contact(&self) -> Option<&Contact> {
        self.entity_id
            .map(|id| &self.contacts.get(&id).unwrap().view)
    }

    fn player_contact_mut(&mut self) -> Option<&mut NetworkContact> {
        self.entity_id
            .map(move |id| self.contacts.get_mut(&id).unwrap())
    }

    fn camera(&self) -> (Vec2, f32) {
        let camera = if let Some(player_contact) = self.player_contact() {
            player_contact.transform().position
        } else {
            self.saved_camera
                .map(|camera| camera.0)
                .unwrap_or(Vec2::ZERO)
        };

        let aspect = self.renderer.aspect();

        let effective_zoom = if aspect > 1.0 {
            self.zoom * aspect
        } else {
            self.zoom
        };

        (camera, effective_zoom)
    }

    /// Interpolates the zoom level closer as if delta_seconds elapsed.
    /// If the player's ship exists, it's camera info is cached, such that it may be returned
    /// even after that ship sinks.
    fn update_camera(&mut self, delta_seconds: f32) {
        let zoom = if let Some(player_contact) = self.player_contact() {
            let camera = player_contact.transform().position;

            // Reduce visual range to to fill more of screen with visual field.
            let zoom = player_contact
                .entity_type()
                .unwrap()
                .data()
                .sensors
                .visual
                .range
                * 0.75;

            self.saved_camera = Some((camera, zoom));
            zoom
        } else if let Some(saved_camera) = self.saved_camera {
            saved_camera.1
        } else {
            300.0
        } * self.input.zoom();

        self.zoom += (zoom - self.zoom) * (6.0 * delta_seconds).min(1.0);
    }

    /// Gets the position of the mouse cursor in world coordinates, given camera info.
    fn mouse_world_position(&self, camera: Vec2, zoom: f32) -> Vec2 {
        camera + self.input.mouse_position * zoom
    }

    /// Sends commands to the server to spawn the player.
    /// Does nothing if there is no session or name is empty.
    pub fn spawn(&mut self, name: String, entity_type: EntityType) {
        if self.server_web_socket.is_none() || trim_spaces(&name).len() == 0 {
            return;
        }

        self.core_web_socket.send(ClientRequest::IdentifySession {
            alias: PlayerAlias::new(&name),
        });

        self.server_web_socket
            .as_mut()
            .unwrap()
            .send(Command::Spawn(Spawn { entity_type }));
    }

    /// Sets the altitude target that will be sent to the server, and plays
    /// the corresponding sound (provided the player's boat is a submarine).
    pub fn set_altitude_target(&mut self, target: Altitude) {
        if let Some(contact) = self.player_contact() {
            if contact.data().sub_kind == EntitySubKind::Submarine {
                if !self.input.altitude_target.is_submerged() && target.is_submerged() {
                    self.audio_player.play("dive");
                } else if self.input.altitude_target.is_submerged() && !target.is_submerged() {
                    self.audio_player.play("surface");
                }
            }
        }

        self.input.altitude_target = target;
    }

    /// Sets the active sensors flag that will be sent to the server,
    /// playing a sound if appropriate.
    pub fn set_active(&mut self, active: bool) {
        if let Some(contact) = self.player_contact() {
            if active && contact.data().sensors.sonar.range >= 0.0 {
                self.audio_player.play("sonar1")
            }
        }
        self.input.active = active;
    }

    /// Sends a command to the server to upgrade the player's ship
    /// to a given entity type, playing a corresponding sound.
    pub fn upgrade(&mut self, entity_type: EntityType) {
        self.audio_player.play("upgrade");
        self.server_web_socket
            .as_mut()
            .unwrap()
            .send(Command::Upgrade(Upgrade { entity_type }))
    }

    /// Sends a command to the server to send a chat message.
    pub fn send_chat(&mut self, message: String, whisper: bool) {
        self.core_web_socket
            .send(ClientRequest::SendChat { message, whisper });
    }

    /// Sends a command to the server to create a new team.
    pub fn create_team(&mut self, team_name: TeamName) {
        self.core_web_socket
            .send(ClientRequest::CreateTeam { team_name });
    }

    /// Sends a command to the server to request joining an
    /// existing team.
    pub fn request_join_team(&mut self, team_id: TeamId) {
        self.core_web_socket
            .send(ClientRequest::RequestJoin { team_id })
    }

    /// Sends a command to the server to accept another player
    /// into a team of which the current player is the captain.
    pub fn accept_join_team(&mut self, player_id: PlayerId) {
        self.core_web_socket
            .send(ClientRequest::AcceptPlayer { player_id });
    }

    /// Sends a command to the server to reject another player
    /// from joining a team of which the current player is the captain.
    pub fn reject_join_team(&mut self, player_id: PlayerId) {
        self.core_web_socket
            .send(ClientRequest::RejectPlayer { player_id });
    }

    /// Sends a command to the server to kick another player from
    /// the team of which the current player is the captain.
    pub fn kick_from_team(&mut self, player_id: PlayerId) {
        self.core_web_socket
            .send(ClientRequest::KickPlayer { player_id });
    }

    /// Sends a command to the server to remove the current player from their current team.
    pub fn leave_team(&mut self) {
        self.core_web_socket.send(ClientRequest::QuitTeam);
    }

    /// Sends a command to the server to mute or un-mute another player.
    pub fn mute_player(&mut self, player_id: PlayerId, mute: bool) {
        self.core_web_socket.send(ClientRequest::MuteSender {
            enable: mute,
            player_id,
        })
    }

    /// Performs time_seconds of game logic, and renders a frame of game state.
    pub fn frame(&mut self, time_seconds: f32) {
        if self.core_web_socket.is_closed() {
            self.players.clear();
            self.teams.clear();
            // TODO: Investigate whether this is a good or bad idea.
            //self.created_invitation_id.clear();
            self.liveboard.clear();
            // No real benefit to hiding these, as they don't change often.
            //self.leaderboards.clear();
            self.joins.clear();
            self.joiners.clear();
            self.chats.clear();
        }

        self.core_web_socket.reconnect_if_necessary(time_seconds);

        for update in self.core_web_socket.receive_updates().into_iter() {
            match update {
                ClientUpdate::CaptainAssigned{..} => {}
                ClientUpdate::ChatSent{..} => {}
                ClientUpdate::InvitationCreated{invitation_id} => {
                    self.created_invitation_id = Some(invitation_id);
                }
                ClientUpdate::JoinRequested{..} => {}
                ClientUpdate::JoinsUpdated{added, removed} => {
                    for &team_id in added.iter() {
                        self.joins.insert(team_id);
                    }
                    for remove in removed.iter() {
                        self.joins.remove(remove);
                    }
                }
                ClientUpdate::JoinersUpdated { added, removed } => {
                    for &player_id in added.iter() {
                        self.joiners.insert(player_id);
                    }
                    for remove in removed.iter() {
                        self.joiners.remove(remove);
                    }
                }
                ClientUpdate::LiveboardUpdated { liveboard }  => {
                    self.liveboard = liveboard.iter().cloned().collect();
                }
                ClientUpdate::LeaderboardUpdated { leaderboard, period } => {
                    self.leaderboards.insert(period, leaderboard.iter().cloned().collect());
                }
                ClientUpdate::MessagesUpdated { added } => {
                    for chat in added.iter() {
                        self.chats.push_back(chat.clone());
                    }
                    while self.chats.len() > 10 {
                        self.chats.pop_front();
                    }
                }
                ClientUpdate::SenderMuted {..} => {}
                ClientUpdate::PlayerAccepted{..} => {}
                ClientUpdate::PlayerKicked{..} => {}
                ClientUpdate::PlayerRejected{..} => {}
                ClientUpdate::PlayersUpdated { count, added, removed } => {
                    self.player_count = count;
                    for player in added.iter() {
                        self.players.insert(player.player_id, player.clone());
                    }
                    for remove in removed.iter() {
                        self.players.remove(remove);
                    }
                }
                ClientUpdate::RegionsUpdated{..} => {}
                ClientUpdate::SessionCreated {
                    session_id,
                    arena_id,
                    server_id,
                    ..  // TODO: copy language and region to UI's drop down.
                } => {
                    if Some(session_id) != self.session_id {
                        // Create an invitation so that the user doesn't have to wait for one later.
                        self.core_web_socket.send(ClientRequest::CreateInvitation);

                        if let Some(socket) = self.server_web_socket.as_mut() {
                            socket.close();
                        }

                        self.arena_id = Some(arena_id);
                        self.session_id = Some(session_id);
                        crate::set_session_id(Some(arena_id.0.to_string()), Some(session_id.0.to_string()));

                        self.server_web_socket = Some(ReconnWebSocket::new(&format!(
                            "{}://{}/ws/{}/",
                            ws_protocol(),
                            if let Some(server_id) = server_id { format!("{}.{}", server_id.0, domain_name()) } else { host() },
                            serde_json::to_string(self.session_id.as_ref().unwrap()).unwrap()
                        ), WebSocketFormat::Binary, None));
                    }
                }
                ClientUpdate::SessionIdentified { .. } => {}
                ClientUpdate::SurveySubmitted => {}
                ClientUpdate::TeamCreated { .. } => {}
                ClientUpdate::TeamQuit => {}
                ClientUpdate::TeamsUpdated { added, removed } => {
                    for team in added.iter() {
                        self.teams.insert(team.team_id, team.clone());
                    }
                    for remove in removed.iter() {
                        self.teams.remove(remove);
                    }
                }
                ClientUpdate::Traced => {}
            }
        }

        if let Some(server_web_socket) = self.server_web_socket.as_mut() {
            if server_web_socket.is_closed() {
                // Clear various data that is only valid when a connection is open. This will cause the
                // splash screen to display with a message saying connection lost.
                self.contacts.clear();
                self.entity_id = None;
                self.score = 0;
                self.saved_camera = None;

                server_web_socket.reconnect_if_necessary(time_seconds);
            } else {
                for update in server_web_socket.receive_updates().into_iter() {
                    self.update(update);
                }
            }
        }

        if !self.audio_player.is_playing("ocean") {
            self.audio_player.play_looping("ocean");
        }

        // Don't let this be negative, or assumptions will be broken.
        let delta_seconds = (time_seconds - self.last_time_seconds).clamp(0.005, 0.5);
        self.last_time_seconds = time_seconds;

        // The distance from player's boat to the closest visible member of each team, for the purpose of sorting and
        // filtering.
        let mut team_proximity: HashMap<TeamId, f32> = HashMap::new();

        // Temporary (will be recalculated after moving ships).
        self.update_camera(delta_seconds);
        let (camera, _) = self.camera();

        // A subset of game logic.
        for NetworkContact {
            model, view, error, ..
        } in &mut self.contacts.values_mut()
        {
            if model
                .entity_type()
                .map(|e| e.data().kind == EntityKind::Boat)
                .unwrap_or(false)
            {
                // Update team_proximity.
                if let Some(player_id) = model.player_id() {
                    if let Some(player) = self.players.get(&player_id) {
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

            //crate::console_log!("err: {}, pos: {}, dir: {}, vel: {}", *error, model.transform().position.distance_squared(view.transform().position) * 0.01, (model.transform().direction - view.transform().direction).abs().to_radians(), model.transform().velocity.difference(view.transform().velocity).to_mps());
            *error = (*error
                + model
                    .transform()
                    .position
                    .distance_squared(view.transform().position)
                    * 0.1
                + (model.transform().direction - view.transform().direction)
                    .abs()
                    .to_radians()
                + model
                    .transform()
                    .velocity
                    .difference(view.transform().velocity)
                    .to_mps()
                    * 0.02
                - 0.1)
                .clamp(0.0, 10.0);

            // Don't interpolate view's guidance if this is the player's boat, so that it doesn't jerk around.
            view.interpolate_towards(
                model,
                Some(model.id()) != self.entity_id,
                delta_seconds * (*error),
            );
            for contact in [model, view] {
                if let Some(entity_type) = contact.entity_type() {
                    let guidance = *contact.guidance();
                    let max_speed = match entity_type.data().sub_kind {
                        // Wait until risen to surface.
                        EntitySubKind::Missile | EntitySubKind::Rocket | EntitySubKind::Sam
                            if contact.altitude().is_submerged() =>
                        {
                            EntityData::SURFACING_PROJECTILE_SPEED_LIMIT
                        }
                        _ => f32::INFINITY,
                    };

                    contact.transform_mut().apply_guidance(
                        entity_type.data(),
                        guidance,
                        max_speed,
                        delta_seconds,
                    );
                }
                contact.transform_mut().do_kinematics(delta_seconds);
            }
        }

        // The player's boat may have moved, so get the camera again.
        let (camera, zoom) = self.camera();
        self.renderer.reset(camera, zoom);
        let mouse_world_position = self.mouse_world_position(camera, zoom);

        // Both width and height must be odd numbers so there is an equal distance from the center
        // on both sides.
        let terrain_width: usize = 2 * ((zoom / terrain::SCALE).max(2.0) as usize + 1) + 3;
        let terrain_height =
            2 * ((zoom / (self.renderer.aspect() * terrain::SCALE)).max(2.0) as usize + 1) + 3;

        let mut terrain_bytes = Vec::with_capacity(terrain_width * terrain_height);
        let terrain_center = Coord::from_position(camera).unwrap();

        for j in 0..terrain_height {
            for i in 0..terrain_width {
                let x = terrain_center.0 as isize + (i as isize - (terrain_width / 2) as isize);
                let y = terrain_center.1 as isize + (j as isize - (terrain_height / 2) as isize);

                terrain_bytes.push(
                    if x >= 0 && x < terrain::SIZE as isize && y >= 0 && y < terrain::SIZE as isize
                    {
                        self.terrain.at(Coord(x as usize, y as usize))
                    } else {
                        255
                    },
                );
            }
        }

        let terrain_offset = Mat3::from_translation(-terrain_center.corner());
        let terrain_scale = &Mat3::from_scale(vec2(
            1.0 / (terrain_width as f32 * terrain::SCALE),
            1.0 / (terrain_height as f32 * terrain::SCALE),
        ));

        // This matrix converts from world space to terrain texture UV coordinates.
        let terrain_matrix = Mat3::from_translation(vec2(0.5, 0.5))
            .mul_mat3(&terrain_scale.mul_mat3(&terrain_offset));

        Texture::realloc_from_bytes(
            &mut self.terrain_texture,
            &self.renderer.gl,
            terrain_width as u32,
            terrain_height as u32,
            &terrain_bytes,
        );

        let (visual_range, visual_restriction) = if let Some(player_contact) = self.player_contact()
        {
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

        self.renderer.render_background(
            self.terrain_texture.as_ref().unwrap(),
            &terrain_matrix,
            camera,
            visual_range,
            visual_restriction,
            self.world_radius,
            time_seconds,
        );

        // Prepare to sort sprites.
        let mut sortable_sprites = Vec::with_capacity(self.contacts.len() * 5);

        // Queue sprites to the end so they get drawn on top.
        let mut text_queue = Vec::new();

        fn add_sortable_sprite<'a>(
            sortable_sprites: &mut Vec<(&'a str, Option<usize>, Vec2, Transform, f32, f32)>,
            sprite: &'a str,
            frame: Option<usize>,
            dimensions: Vec2,
            transform: Transform,
            altitude: f32,
            alpha: f32,
        ) {
            sortable_sprites.push((sprite, frame, dimensions, transform, altitude, alpha));
        }

        fn add_sortable_entity(
            sortable_sprites: &mut Vec<(&str, Option<usize>, Vec2, Transform, f32, f32)>,
            entity_type: EntityType,
            transform: Transform,
            altitude: f32,
            alpha: f32,
        ) {
            add_sortable_sprite(
                sortable_sprites,
                entity_type.to_str(),
                None,
                entity_type.data().dimensions(),
                transform,
                altitude,
                alpha,
            );
        }

        // Update animations.
        let mut i = 0;
        while i < self.animations.len() {
            let animation = &mut self.animations[i];
            animation.update(delta_seconds);

            let len = self
                .renderer
                .sprite_sheet
                .animations
                .get(animation.name)
                .unwrap()
                .len();

            if animation.frame >= len {
                self.animations.swap_remove(i);
            } else {
                add_sortable_sprite(
                    &mut sortable_sprites,
                    animation.name,
                    Some(animation.frame),
                    Vec2::splat(animation.scale),
                    Transform::from_position(animation.position),
                    animation.altitude,
                    1.0,
                );
                i += 1;
            }
        }

        for NetworkContact { view: contact, .. } in self.contacts.values() {
            let friendly = Self::is_friendly(self.player_id, contact, &self.players);

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

                add_sortable_entity(
                    &mut sortable_sprites,
                    entity_type,
                    *contact.transform(),
                    altitude,
                    alpha,
                );
                let data: &'static EntityData = entity_type.data();
                if contact.is_boat() && !contact.reloads().is_empty() {
                    for i in 0..data.armaments.len() {
                        let armament = &data.armaments[i];
                        if armament.hidden || armament.vertical || !(armament.external || friendly)
                        {
                            continue;
                        }
                        add_sortable_entity(
                            &mut sortable_sprites,
                            armament.entity_type,
                            *contact.transform() + data.armament_transform(contact.turrets(), i),
                            altitude + 0.02,
                            alpha
                                * (if contact.reloads()[i] == Ticks::ZERO {
                                    1.0
                                } else {
                                    0.5
                                }),
                        );
                    }
                }
                for (i, turret) in data.turrets.iter().enumerate() {
                    if let Some(entity_type) = turret.entity_type {
                        add_sortable_entity(
                            &mut sortable_sprites,
                            entity_type,
                            *contact.transform()
                                + Transform {
                                    position: turret.position(),
                                    direction: contact.turrets()[i],
                                    velocity: Velocity::ZERO,
                                }
                                + Transform {
                                    position: entity_type.data().offset(),
                                    direction: Angle::ZERO,
                                    velocity: Velocity::ZERO,
                                },
                            altitude + 0.01,
                            alpha,
                        );
                    }
                }

                // GUI overlays.
                let overlay_vertical_position = data.radius * 1.2;

                match data.kind {
                    EntityKind::Boat => {
                        // Is this player's own boat?
                        if self.player_id.is_some() && contact.player_id() == self.player_id {
                            // Radii
                            let hud_color = rgba(255, 255, 255, 255 / 3);
                            let hud_thickness = 0.0025 * zoom;

                            // Throttle rings.
                            // 1. Inner
                            self.renderer.add_circle_graphic(
                                contact.transform().position,
                                data.radii().start,
                                hud_thickness,
                                hud_color,
                            );
                            // 2. Outer
                            self.renderer.add_circle_graphic(
                                contact.transform().position,
                                data.radii().end,
                                hud_thickness,
                                hud_color,
                            );
                            // 3. Actual speed
                            self.renderer.add_circle_graphic(
                                contact.transform().position,
                                map_ranges(
                                    contact.transform().velocity.abs().to_mps(),
                                    0.0..data.speed.to_mps(),
                                    data.radii(),
                                    false,
                                ),
                                hud_thickness,
                                hud_color,
                            );
                            // 4. Target speed
                            self.renderer.add_circle_graphic(
                                contact.transform().position,
                                map_ranges(
                                    contact.guidance().velocity_target.abs().to_mps(),
                                    0.0..data.speed.to_mps(),
                                    data.radii(),
                                    true,
                                ),
                                hud_thickness,
                                hud_color,
                            );

                            // Target bearing line.
                            let dir_mat =
                                Mat2::from_angle(contact.guidance().direction_target.to_radians());
                            self.renderer.add_line_graphic(
                                contact.transform().position
                                    + dir_mat * vec2(data.radii().start, 0.0),
                                contact.transform().position
                                    + dir_mat * vec2(data.radii().end, 0.0),
                                hud_thickness,
                                hud_color,
                            );

                            // Turret azimuths
                            if let Some(i) = self.find_best_armament(contact, false) {
                                let armament = &data.armaments[i];
                                if armament.entity_type != EntityType::Depositor {
                                    if let Some(turret_index) = armament.turret {
                                        let turret = &data.turrets[turret_index];
                                        let contact_direction = contact.transform().direction;
                                        let transform = *contact.transform()
                                            + Transform::from_position(turret.position());

                                        let inner: f32 = 0.2 * data.width;
                                        let outer: f32 = 0.325 * data.width;
                                        let span: f32 = outer - inner;
                                        let middle: f32 = inner + span * 0.5;
                                        let thickness = hud_thickness * 2.0;
                                        let color = hud_color;

                                        let azimuth_line = |renderer: &mut Renderer, angle: f32| {
                                            let dir_mat = Mat2::from_angle(angle);
                                            renderer.add_line_graphic(
                                                transform.position + dir_mat * vec2(inner, 0.0),
                                                transform.position + dir_mat * vec2(outer, 0.0),
                                                thickness,
                                                color,
                                            );
                                        };

                                        azimuth_line(
                                            &mut self.renderer,
                                            (contact_direction + contact.turrets()[turret_index])
                                                .to_radians(),
                                        );
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
                                        if turret.azimuth_br != Angle::ZERO
                                            || turret.azimuth_bl != Angle::ZERO
                                        {
                                            azimuth_line(&mut self.renderer, right_back);
                                            azimuth_line(&mut self.renderer, left_back);
                                        }

                                        if turret.azimuth_fr != Angle::ZERO
                                            || turret.azimuth_fl != Angle::ZERO
                                        {
                                            azimuth_line(&mut self.renderer, left_front);
                                            azimuth_line(&mut self.renderer, right_front);
                                        }

                                        if turret.azimuth_fr + turret.azimuth_br < Angle::PI {
                                            self.renderer.add_arc_graphic(
                                                transform.position,
                                                middle,
                                                right_back..if right_front > right_back {
                                                    right_front
                                                } else {
                                                    right_front + 2.0 * std::f32::consts::PI
                                                },
                                                span,
                                                color,
                                            );
                                        }
                                        if turret.azimuth_fl + turret.azimuth_bl < Angle::PI {
                                            self.renderer.add_arc_graphic(
                                                transform.position,
                                                middle,
                                                left_front..if left_back > left_front {
                                                    left_back
                                                } else {
                                                    left_back + 2.0 * std::f32::consts::PI
                                                },
                                                span,
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
                            let health_back_position =
                                contact.transform().position + vec2(0.0, overlay_vertical_position);
                            let health_bar_position = health_back_position
                                + vec2(
                                    -health_bar_width * 0.5 + health * health_bar_width * 0.5,
                                    0.0,
                                );
                            self.renderer.add_rectangle_graphic(
                                health_back_position,
                                vec2(health_bar_width, health_bar_height),
                                0.0,
                                rgba(85, 85, 85, 127),
                            );
                            self.renderer.add_rectangle_graphic(
                                health_bar_position,
                                vec2(health * health_bar_width, health_bar_height),
                                0.0,
                                color.extend(1.0),
                            );
                        }

                        // Name
                        let text = if let Some(player) =
                            self.players.get(contact.player_id().as_ref().unwrap())
                        {
                            if let Some(team) =
                                player.team_id.and_then(|team_id| self.teams.get(&team_id))
                            {
                                format!("[{}] {}", team.team_name, player.alias)
                            } else {
                                player.alias.0.to_string()
                            }
                        } else {
                            // This is not meant to happen in production. It is for debugging.
                            format!("{}", contact.player_id().unwrap().0.get())
                        };

                        text_queue.push((
                            contact.transform().position
                                + vec2(0.0, overlay_vertical_position + 0.035 * zoom),
                            0.035 * zoom,
                            color.extend(1.0),
                            text,
                        ));
                    }
                    EntityKind::Weapon | EntityKind::Decoy | EntityKind::Aircraft => {
                        let triangle_position =
                            contact.transform().position + vec2(0.0, overlay_vertical_position);
                        self.renderer.add_triangle_graphic(
                            triangle_position + vec2(0.0, 0.01 * zoom),
                            Vec2::splat(0.02 * zoom),
                            180f32.to_radians(),
                            color.extend(1.0),
                        );
                    }
                    _ => {}
                }

                // Add particles.
                let direction_vector: Vec2 = contact.transform().direction.into();
                let tangent_vector = direction_vector.perp();
                let amount = (data.width * (1.0 / 7.5)) as usize + 1;
                let mut rng = thread_rng();

                // Wake/trail particles.
                if contact.transform().velocity != Velocity::ZERO
                    && (data.sub_kind != EntitySubKind::Submarine
                        || contact.transform().velocity
                            > Velocity::from_mps(EntityData::CAVITATION_VELOCITY))
                {
                    for _ in 0..amount {
                        let collection = if contact.altitude().is_airborne() {
                            &mut self.airborne_particles
                        } else {
                            &mut self.sea_level_particles
                        };
                        collection.push(Particle {
                            position: contact.transform().position
                                - direction_vector * (data.length * 0.485)
                                + tangent_vector * (data.width * (rng.gen::<f32>() - 0.5) * 0.25),
                            velocity: direction_vector
                                * contact.transform().velocity.to_mps()
                                * 0.75,
                            color: 1.0,
                            created: time_seconds,
                        });
                    }
                }

                // Exhaust particles
                if !contact.altitude().is_submerged() {
                    for exhaust in data.exhausts.iter() {
                        for _ in 0..amount * 2 {
                            self.airborne_particles.push(Particle {
                                position: contact.transform().position
                                    + direction_vector * exhaust.position_forward
                                    + tangent_vector * exhaust.position_side
                                    + gen_radius(&mut rng, 1.5),
                                velocity: gen_radius(&mut rng, 6.0),
                                color: if entity_type == EntityType::OilPlatform {
                                    -1.0
                                } else {
                                    0.4
                                },
                                created: time_seconds,
                            });
                        }
                    }
                }
            } else {
                self.renderer.render_sprite(
                    "contact",
                    None,
                    Vec2::splat(10.0),
                    *contact.transform(),
                    1.0,
                );
            }
        }

        // Sort sprites by altitude.
        sortable_sprites.sort_unstable_by(|a, b| a.4.partial_cmp(&b.4).unwrap());
        for sprite in sortable_sprites {
            self.renderer
                .render_sprite(sprite.0, sprite.1, sprite.2, sprite.3, sprite.5);
        }

        // Calculate powf once for particles.
        let powf_0_25_seconds = 0.25f32.powf(delta_seconds);

        // Update sea-level particles.
        let mut i = 0;
        while i < self.sea_level_particles.len() {
            if self.sea_level_particles[i].update(delta_seconds, powf_0_25_seconds) {
                self.sea_level_particles.swap_remove(i);
            } else {
                let particle = &self.sea_level_particles[i];
                self.renderer
                    .add_particle(particle.position, particle.color, particle.created);
                i += 1;
            }
        }
        self.renderer.render_particles(time_seconds);

        // Render sprites in between sea level particles and airborne particles.
        self.renderer.render_sprites();

        // Update airborne particles.
        let mut i = 0;
        while i < self.airborne_particles.len() {
            let particle = &mut self.airborne_particles[i];

            // Apply wind.
            particle.velocity += vec2(14.0, 3.0) * delta_seconds;

            particle.update(delta_seconds, powf_0_25_seconds);

            if time_seconds >= particle.created + 1.5 {
                self.airborne_particles.swap_remove(i);
            } else {
                self.renderer
                    .add_particle(particle.position, particle.color, particle.created);
                i += 1;
            }
        }
        self.renderer.render_particles(time_seconds);

        self.renderer.render_graphics();

        for (position, scale, color, text) in text_queue {
            let name_texture = self.text_cache.get(&self.renderer.gl, &text);
            self.renderer
                .render_text(position, scale, color, name_texture);
        }

        // Buffer until later so as not to borrow the websocket early. The web socket's lifetime is tied
        // to the deserialized updates it produces (because serde_json can reference the raw json buffer).
        let mut to_send = Vec::with_capacity(3);
        let mut reset_input = false;

        let status = if let Some(player_contact) = self.player_contact() {
            let direction_target =
                Angle::from_atan2(self.input.mouse_position.y, self.input.mouse_position.x);

            if time_seconds > self.last_control_seconds + Self::CONTROL_PERIOD {
                let mut guidance = None;

                if self.input.mouse_right_down
                    || self.input.mouse_left_down_not_click()
                    || self.input.joystick.is_some()
                    || self.input.stop
                {
                    let max_speed = player_contact.data().speed.to_mps();

                    if let Some(joystick) = self.input.joystick {
                        guidance = Some(Guidance {
                            direction_target: player_contact.transform().direction
                                + Angle::from_radians(0.5 * joystick.x),
                            velocity_target: if self.input.stop {
                                Velocity::ZERO
                            } else if joystick.y.abs() > 0.05 {
                                player_contact.transform().velocity
                                    + Velocity::from_mps(0.25 * max_speed * joystick.y)
                            } else {
                                player_contact.guidance().velocity_target
                            },
                        })
                    };

                    if self.input.mouse_right_down || self.input.mouse_left_down_not_click() {
                        guidance = Some(Guidance {
                            // Limit turning while "stopped"
                            direction_target: if self.input.stop {
                                player_contact.transform().direction
                                    + (direction_target - player_contact.transform().direction)
                                        .clamp_magnitude(Angle::from_radians(0.5))
                            } else {
                                direction_target
                            },
                            velocity_target: if self.input.stop {
                                Velocity::ZERO
                            } else {
                                Velocity::from_mps(util::map_ranges(
                                    mouse_world_position
                                        .distance(player_contact.transform().position),
                                    player_contact.data().radii(),
                                    0.0..max_speed,
                                    true,
                                ))
                            },
                        });
                    };
                }

                to_send.push(Command::Control(Control {
                    guidance,
                    angular_velocity_target: None,
                    altitude_target: if player_contact.data().sub_kind == EntitySubKind::Submarine {
                        Some(self.input.altitude_target)
                    } else {
                        None
                    },
                    aim_target: Some(mouse_world_position),
                    active: self.input.active,
                }));

                if self.input.pay {
                    to_send.push(Command::Pay(Pay {
                        position: mouse_world_position,
                    }));
                }

                if self.input.mouse_left_click || self.input.shoot {
                    if let Some(i) = self.find_best_armament(player_contact, true) {
                        to_send.push(Command::Fire(Fire {
                            index: i as u8,
                            position_target: mouse_world_position,
                        }));
                    }
                }

                reset_input = true;
            }

            let status = Status::Alive {
                entity_type: player_contact.entity_type().unwrap(),
                position: player_contact.transform().position.into(),
                direction: player_contact.transform().direction,
                velocity: player_contact.transform().velocity,
                altitude: player_contact.altitude(),
                armament_consumption: Some(player_contact.reloads().into()), // TODO fix to clone arc
            };

            for command in to_send.into_iter() {
                if let Command::Control(control) = &command {
                    self.last_control_seconds = time_seconds;

                    // Predict control outcome on boat, which exists if control is being sent.
                    let boat = self.player_contact_mut().unwrap();
                    boat.model.predict_control(control);
                    boat.view.predict_control(control);
                }
                self.server_web_socket.as_mut().unwrap().send(command);
            }

            status
        } else {
            Status::Spawning {
                death_reason: self.death_reason.as_ref().map(|reason| match reason {
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
                            self.players
                                .get(player_id)
                                .map(|p| p.alias)
                                .unwrap_or(PlayerAlias::new("???")),
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
                            self.players
                                .get(player_id)
                                .map(|p| p.alias)
                                .unwrap_or(PlayerAlias::new("???")),
                        ),
                        entity: None,
                    },
                    DeathReason::Weapon(player_id, entity_type) => DeathReasonModel {
                        death_type: "sinking",
                        player: Some(
                            self.players
                                .get(player_id)
                                .map(|p| p.alias)
                                .unwrap_or(PlayerAlias::new("???")),
                        ),
                        entity: Some(*entity_type),
                    },
                    _ => panic!("invalid death reason for boat: {:?}", reason),
                }),
                connection_lost: self
                    .server_web_socket
                    .as_ref()
                    .map(|sock| sock.is_closed())
                    .unwrap_or(false),
            }
        };

        let team_id = self
            .player_id
            .and_then(|p| self.players.get(&p))
            .and_then(|p| p.team_id);

        crate::set_state(&State {
            player_id: self.player_id,
            team_name: team_id
                .and_then(|id| self.teams.get(&id))
                .map(|t| t.team_name),
            invitation_id: self.created_invitation_id,
            score: self.score,
            player_count: self.player_count,
            status,
            chats: self
                .chats
                .iter()
                .filter_map(|chat| {
                    Some(ChatModel {
                        name: chat.alias.clone(),
                        player_id: chat.player_id,
                        team: chat.team_name.clone(),
                        message: chat.text.clone(),
                        whisper: chat.whisper,
                    })
                })
                .collect(),
            liveboard: self
                .liveboard
                .iter()
                .filter_map(|item| {
                    let player = self.players.get(&item.player_id);
                    if let Some(player) = player {
                        let team_name = player
                            .team_id
                            .and_then(|team_id| self.teams.get(&team_id))
                            .map(|team| team.team_name);
                        Some(LeaderboardItemModel {
                            name: player.alias.clone(),
                            team: team_name,
                            score: item.score,
                        })
                    } else {
                        None
                    }
                })
                .collect(),
            leaderboards: self
                .leaderboards
                .iter()
                .map(|(period, leaderboard)| {
                    (
                        *period,
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
            team_members: if team_id.is_some() {
                self.players
                    .values()
                    .filter(|p| p.team_id == team_id)
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
            team_captain: team_id.is_some()
                && self
                    .player_id
                    .and_then(|id| self.players.get(&id))
                    .map(|p| p.team_captain)
                    .unwrap_or(false),
            team_join_requests: self
                .joiners
                .iter()
                .filter_map(|id| {
                    self.players.get(id).map(|player| TeamPlayerModel {
                        player_id: player.player_id,
                        name: player.alias.clone(),
                        captain: false,
                    })
                })
                .collect(),
            teams: self
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
                    joining: self.joins.contains(team_id),
                })
                .take(5)
                .collect(),
        });

        if reset_input {
            self.input.reset();
        }
        self.text_cache.tick();

        if let Some(fps) = self.fps_monitor.update(delta_seconds) {
            if !self.core_web_socket.is_closed() {
                self.core_web_socket.send(ClientRequest::TallyFps { fps });
            }
        }
    }

    /// Finds the best armament (i.e. the one that will be fired if the mouse is clicked).
    /// Armaments are scored by a combination of distance and angle to target.
    fn find_best_armament(&self, player_contact: &Contact, angle_limit: bool) -> Option<usize> {
        let (camera, zoom) = self.camera();
        let mouse_world_position = self.mouse_world_position(camera, zoom);

        // The f32 represents how good the shot is, lower is better.
        let mut best_armament: Option<(usize, f32)> = None;

        if let Some(armament_selection) = self.input.armament_selection {
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

                let armament_direction_target =
                    Angle::from(mouse_world_position - transform.position);

                let mut angle_diff = (armament_direction_target - transform.direction).abs();
                let distance_squared = mouse_world_position.distance_squared(transform.position);
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
