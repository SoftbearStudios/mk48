// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apply::Apply;
use crate::context::{Context, ServerState};
use crate::fps_monitor::FpsMonitor;
use crate::frontend::Frontend;
use crate::game_client::GameClient;
use crate::js_hooks::canvas;
use crate::keyboard::{Key, KeyboardEvent as GameClientKeyboardEvent};
use crate::local_storage::LocalStorage;
use crate::mouse::{MouseButton, MouseButtonState, MouseEvent as GameClientMouseEvent};
use crate::reconn_web_socket::ReconnWebSocket;
use crate::renderer::renderer::Renderer;
use crate::setting::CommonSettings;
use crate::setting::Settings;
use common_util::range::map_ranges;
use core_protocol::id::{PlayerId, ServerId, TeamId};
use core_protocol::name::TeamName;
use core_protocol::rpc::{
    ChatRequest, ClientRequest, ClientUpdate, InvitationRequest, PlayerRequest, Request,
    TeamRequest, Update,
};
use core_protocol::web_socket::WebSocketProtocol;
use glam::{IVec2, Vec2};
use std::panic;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{FocusEvent, HtmlInputElement, KeyboardEvent, MouseEvent, TouchEvent, WheelEvent};

pub struct Infrastructure<G: GameClient> {
    game: G,
    context: Context<G>,
    renderer: Renderer,
    renderer_layer: G::RendererLayer,
    statistic_fps_monitor: FpsMonitor,
}

impl<G: GameClient> Infrastructure<G> {
    pub fn new(mut game: G, frontend: Box<dyn Frontend<G::UiProps> + 'static>) -> Self {
        panic::set_hook(Box::new(console_error_panic_hook::hook));

        // First load local storage common settings.
        // Not guaranteed to set either or both to Some. Could fail to load.
        let local_storage = LocalStorage::new();
        let common_settings = CommonSettings::load(&local_storage, CommonSettings::default());

        // Next create renderer and load game settings with it.
        let mut renderer = Renderer::new(common_settings.antialias);
        let game_settings =
            G::GameSettings::load(&local_storage, game.init_settings(&mut renderer));

        // Finally create context with common and game settings.
        let mut context = Context::new(local_storage, common_settings, game_settings, frontend);
        let renderer_layer = game.init_layer(&mut renderer, &mut context);

        Self {
            game,
            context,
            renderer,
            renderer_layer,
            statistic_fps_monitor: FpsMonitor::new(60.0),
        }
    }

    pub fn frame(&mut self, time_seconds: f32) {
        let elapsed_seconds = (time_seconds - self.context.client.update_seconds).clamp(0.001, 0.5);
        self.context.client.update_seconds = time_seconds;

        self.sync_mouse_world_space();

        for inbound in self
            .context
            .socket
            .update(&mut self.context.state, time_seconds)
        {
            if let &Update::Client(ClientUpdate::SessionCreated {
                arena_id,
                session_id,
                server_id,
                ..
            }) = &inbound
            {
                // Create an invitation so that the player doesn't have to wait for one later.
                self.context
                    .send_to_server(Request::Invitation(InvitationRequest::CreateInvitation));

                let (host, server_id) = Context::<G>::compute_websocket_host(
                    &self.context.common_settings,
                    server_id,
                    &*self.context.frontend,
                );
                self.context.socket.reset_host(host);
                self.context
                    .common_settings
                    .set_server_id(server_id, &mut self.context.local_storage);

                if self.context.socket.is_closed()
                    || Some((arena_id, session_id)) != self.context.common_settings.session_tuple()
                {
                    self.context
                        .common_settings
                        .set_arena_id(Some(arena_id), &mut self.context.local_storage);
                    self.context
                        .common_settings
                        .set_session_id(Some(session_id), &mut self.context.local_storage);
                }
            }

            if let Update::Game(update) = &inbound {
                self.game.peek_game(
                    update,
                    &mut self.context,
                    &self.renderer,
                    &mut self.renderer_layer,
                );
            }
            self.context.state.apply(inbound);
        }

        self.renderer.pre_prepare(&mut self.renderer_layer);
        self.game.tick(
            elapsed_seconds,
            &mut self.context,
            &mut self.renderer,
            &mut self.renderer_layer,
        );
        self.renderer.render(&mut self.renderer_layer, time_seconds);

        if let Some(fps) = self.statistic_fps_monitor.update(elapsed_seconds) {
            self.context
                .send_to_server(Request::Client(ClientRequest::TallyFps(fps)));
        }
    }

    pub fn keyboard(&mut self, event: KeyboardEvent) {
        if let Some(target) = event.target() {
            if target.is_instance_of::<HtmlInputElement>() {
                return;
            }
        }

        let type_ = event.type_();
        match type_.as_str() {
            "keydown" | "keyup" => {
                let down = type_ == "keydown";

                if let Some(key) = Key::try_from_key_code(event.key_code()) {
                    let e = GameClientKeyboardEvent {
                        key,
                        ctrl: event.ctrl_key(),
                        down,
                        shift: event.shift_key(),
                        time: self.context.client.update_seconds,
                    };

                    if down {
                        // Simulate zooming.
                        match key {
                            Key::PageDown => self.raw_zoom(1.0),
                            Key::PageUp => self.raw_zoom(-1.0),
                            Key::MinusUnderscore if e.ctrl => self.raw_zoom(1.0),
                            Key::EqualsPlus if e.ctrl => self.raw_zoom(-1.0),
                            _ => {}
                        }
                    }

                    // Don't block CTRL+C, CTRL+V, etc.
                    if !(e.ctrl && matches!(e.key, Key::C | Key::F | Key::R | Key::V | Key::X)) {
                        event.prevent_default();
                        event.stop_propagation();
                    }

                    self.game.peek_keyboard(&e, &mut self.context);
                    self.context.keyboard.apply(e);
                }
            }
            _ => {}
        }
    }

    pub fn keyboard_focus(&mut self, event: FocusEvent) {
        if event.type_() == "blur" {
            self.context.keyboard.reset();
        }
    }

    pub fn mouse(&mut self, event: MouseEvent) {
        let type_ = event.type_();

        match type_.as_str() {
            "mousedown" | "mouseup" => {
                if let Some(button) = MouseButton::try_from_button(event.button()) {
                    let down = type_ == "mousedown";

                    let e = GameClientMouseEvent::Button {
                        button,
                        down,
                        time: self.context.client.update_seconds,
                    };
                    self.game.peek_mouse(&e, &mut self.context, &self.renderer);
                    self.context.mouse.apply(e);
                }
            }
            "mousemove" => {
                self.mouse_move(event.client_x(), event.client_y());
            }
            "mouseleave" => {
                self.context.mouse.reset();
            }
            _ => {}
        }
    }

    pub fn mouse_focus(&mut self, event: FocusEvent) {
        if event.type_() == "blur" {
            self.context.mouse.reset();
        }
    }

    pub fn touch(&mut self, event: TouchEvent) {
        let type_ = event.type_();

        let target_touches = event.target_touches();

        match type_.as_str() {
            "touchstart" | "touchend" => {
                let down = type_ == "touchstart";

                // Any change in touch count invalidates pinch to zoom.
                self.context.mouse.pinch_distance = None;

                if target_touches.length() <= 1 {
                    let e = GameClientMouseEvent::Button {
                        button: MouseButton::Left,
                        down,
                        time: self.context.client.update_seconds,
                    };
                    self.game.peek_mouse(&e, &mut self.context, &self.renderer);
                    self.context.mouse.apply(e);
                } else if self.context.mouse.is_down(MouseButton::Left) {
                    *self.context.mouse.state_mut(MouseButton::Left) = MouseButtonState::Up;
                }
            }
            "touchmove" => {
                event.prevent_default();

                match target_touches.length() {
                    1 => {
                        // Emulate left mouse.
                        let first = target_touches.item(0);
                        if let Some(first) = first {
                            self.mouse_move(first.client_x(), first.client_y());
                            self.context.mouse.pinch_distance = None;
                        } else {
                            debug_assert!(false, "expected 1 touch");
                        }
                    }
                    2 => {
                        // Emulate wheel (pinch to zoom).
                        let first: Option<Vec2> = target_touches
                            .item(0)
                            .map(|t| IVec2::new(t.client_x(), t.client_y()).as_vec2());
                        let second: Option<Vec2> = target_touches
                            .item(1)
                            .map(|t| IVec2::new(t.client_x(), t.client_y()).as_vec2());
                        if let Some((first, second)) = first.zip(second) {
                            let pinch_distance = first.distance(second);

                            if let Some(previous_pinch_distance) = self.context.mouse.pinch_distance
                            {
                                let delta = 0.03 * (previous_pinch_distance - pinch_distance);
                                self.raw_zoom(delta);
                            }

                            self.context.mouse.pinch_distance = Some(pinch_distance);
                        } else {
                            debug_assert!(false, "expected 2 touches");
                        }
                    }
                    _ => {
                        // 0 and >2 touch gestures are not (yet) supported.
                    }
                }
            }
            _ => {}
        }
    }

    /// Creates a mouse wheel event with the given delta.
    pub fn raw_zoom(&mut self, delta: f32) {
        let e = GameClientMouseEvent::Wheel(delta);
        self.game.peek_mouse(&e, &mut self.context, &self.renderer);
        self.context.mouse.apply(e);
    }

    /// Converts page position (from event) to view position (-1..1).
    fn client_coordinate_to_view(x: i32, y: i32) -> Vec2 {
        let rect = canvas().unwrap().get_bounding_client_rect();

        Vec2::new(
            map_ranges(
                x as f32,
                rect.x() as f32..rect.x() as f32 + rect.width() as f32,
                -1.0..1.0,
                false,
            ),
            map_ranges(
                y as f32,
                rect.y() as f32 + rect.height() as f32..rect.y() as f32,
                -1.0..1.0,
                false,
            ),
        )
    }

    pub fn ui_event(&mut self, event: G::UiEvent) {
        self.game
            .peek_ui(&event, &mut self.context, &mut self.renderer_layer);
        self.context.ui.apply(event);
    }

    /// Helper to issue a mouse move event. Takes client coordinates.
    fn mouse_move(&mut self, x: i32, y: i32) {
        let view_position = Self::client_coordinate_to_view(x, y);

        let e = GameClientMouseEvent::MoveViewSpace(view_position);
        self.game.peek_mouse(&e, &mut self.context, &self.renderer);
        self.context.mouse.apply(e);

        // If the mouse moves in view space, it also moves in world space.
        let e2 =
            GameClientMouseEvent::MoveWorldSpace(self.renderer.to_world_position(view_position));
        self.game.peek_mouse(&e2, &mut self.context, &self.renderer);
        self.context.mouse.apply(e2);
    }

    /// Helper to issue a mouse move world space event if needed.
    fn sync_mouse_world_space(&mut self) {
        if let Some(view_position) = self.context.mouse.view_position {
            let world_position = self.renderer.to_world_position(view_position);
            if self.context.mouse.world_position != Some(world_position) {
                let e = GameClientMouseEvent::MoveWorldSpace(world_position);
                self.game.peek_mouse(&e, &mut self.context, &self.renderer);
                self.context.mouse.apply(e);
            }
        }
    }

    pub fn wheel(&mut self, event: WheelEvent) {
        self.raw_zoom(event.delta_y() as f32 * 0.01)
    }

    /// Sends a command to the server to send a chat message.
    pub fn send_chat(&mut self, message: String, whisper: bool) {
        self.context
            .socket
            .send(Request::Chat(ChatRequest::Send { message, whisper }));
    }

    /// Sends a command to the server to create a new team.
    pub fn create_team(&mut self, team_name: TeamName) {
        self.context
            .socket
            .send(Request::Team(TeamRequest::Create(team_name)));
    }

    /// Sends a command to the server to request joining an
    /// existing team.
    pub fn request_join_team(&mut self, team_id: TeamId) {
        self.context
            .socket
            .send(Request::Team(TeamRequest::Join(team_id)))
    }

    /// Sends a command to the server to accept another player
    /// into a team of which the current player is the captain.
    pub fn accept_join_team(&mut self, player_id: PlayerId) {
        self.context
            .socket
            .send(Request::Team(TeamRequest::Accept(player_id)));
    }

    /// Sends a command to the server to reject another player
    /// from joining a team of which the current player is the captain.
    pub fn reject_join_team(&mut self, player_id: PlayerId) {
        self.context
            .socket
            .send(Request::Team(TeamRequest::Reject(player_id)));
    }

    /// Sends a command to the server to kick another player from
    /// the team of which the current player is the captain.
    pub fn kick_from_team(&mut self, player_id: PlayerId) {
        self.context
            .socket
            .send(Request::Team(TeamRequest::Kick(player_id)));
    }

    /// Sends a command to the server to remove the current player from their current team.
    pub fn leave_team(&mut self) {
        self.context.socket.send(Request::Team(TeamRequest::Leave));
    }

    /// Sends a command to the server to report another.
    pub fn report_player(&mut self, player_id: PlayerId) {
        self.context
            .socket
            .send(Request::Player(PlayerRequest::Report(player_id)))
    }

    /// Sends a command to the server to mute or un-mute another player.
    pub fn mute_player(&mut self, player_id: PlayerId, mute: bool) {
        let req = if mute {
            ChatRequest::Mute(player_id)
        } else {
            ChatRequest::Unmute(player_id)
        };
        self.context.socket.send(Request::Chat(req))
    }

    /// Set the websocket protocol of future socket messages.
    pub fn web_socket_protocol(&mut self, protocol: WebSocketProtocol) {
        self.context.socket.set_protocol(protocol);
        self.context
            .common_settings
            .set_protocol(protocol, &mut self.context.local_storage);
    }

    /// Send error message to server.
    pub fn trace(&mut self, message: String) {
        self.context.send_trace(message);
    }

    /// Gets a game or common setting.
    pub fn get_setting(&self, key: &str) -> JsValue {
        let mut ret = self.context.settings.get(key);
        if ret.is_null() {
            ret = self.context.common_settings.get(key);
        }
        ret
    }

    /// Sets a game or common setting.
    pub fn set_setting(&mut self, key: &str, value: JsValue) {
        self.context
            .settings
            .set(key, value.clone(), &mut self.context.local_storage);
        self.context
            .common_settings
            .set(key, value, &mut self.context.local_storage);
    }

    /// Connects to a different server.
    pub fn choose_server_id(&mut self, server_id: Option<ServerId>) {
        if server_id == self.context.common_settings.server_id {
            return;
        }
        // Clear state from old server.
        self.context.state = ServerState::default();

        let (host, server_id) = Context::<G>::compute_websocket_host(
            &self.context.common_settings,
            server_id,
            &*self.context.frontend,
        );
        self.context.socket =
            ReconnWebSocket::new(host, self.context.common_settings.protocol, None);
        self.context
            .common_settings
            .set_server_id(server_id, &mut self.context.local_storage);
    }

    /// Simulates dropping of one or both websockets.
    pub fn simulate_drop_web_socket(&mut self) {
        self.context.socket.simulate_drop();
    }
}
