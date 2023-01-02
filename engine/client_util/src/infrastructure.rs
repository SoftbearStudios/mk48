// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apply::Apply;
use crate::browser_storage::BrowserStorages;
use crate::context::{Context, ServerState};
use crate::fps_monitor::FpsMonitor;
use crate::frontend::Frontend;
use crate::game_client::GameClient;
use crate::keyboard::{Key, KeyboardEvent as GameClientKeyboardEvent};
use crate::mouse::{MouseButton, MouseEvent as GameClientMouseEvent};
use crate::reconn_web_socket::ReconnWebSocket;
use crate::setting::CommonSettings;
use crate::visibility::VisibilityEvent;
use common_util::range::map_ranges;
use core_protocol::id::{PlayerId, ServerId, TeamId};
use core_protocol::name::TeamName;
use core_protocol::rpc::{
    AdType, ChatRequest, ClientRequest, ClientUpdate, InvitationRequest, PlayerRequest, Request,
    TeamRequest, Update,
};
use core_protocol::web_socket::WebSocketProtocol;
use glam::{IVec2, Vec2};
use js_sys::Function;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    Event, FocusEvent, HtmlInputElement, KeyboardEvent, MouseEvent, Touch, TouchEvent, WheelEvent,
};

pub struct Infrastructure<G: GameClient> {
    game: G,
    pub context: Context<G>,
    /// Id of the [`Touch`] associated with the earliest finger to make contact with the touch
    /// screen in a gesture, used to emulate left click.
    left_touch_id: Option<i32>,
    /// Id of the [`Touch`] associated with the second earliest finger to make contact with the touch
    /// screen in a gesture, used to emulate right click.
    right_touch_id: Option<i32>,
    statistic_fps_monitor: FpsMonitor,
}

impl<G: GameClient> Infrastructure<G> {
    pub fn new(
        browser_storages: BrowserStorages,
        common_settings: CommonSettings,
        settings: G::GameSettings,
        frontend: Box<dyn Frontend<G::UiProps> + 'static>,
    ) -> Result<Self, (String, BrowserStorages, CommonSettings, G::GameSettings)> {
        // Don't try to catch panics if aborting (because it's useless).
        #[cfg(panic = "unwind")]
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        #[cfg(feature = "joined")]
        crate::joined::init();

        let context = Context::new(browser_storages, common_settings, settings, frontend);

        match G::new(&context) {
            Ok(game) => Ok(Self {
                game,
                context,
                left_touch_id: None,
                right_touch_id: None,
                statistic_fps_monitor: FpsMonitor::new(60.0),
            }),
            Err(e) => Err((
                e,
                context.browser_storages,
                context.common_settings,
                context.settings,
            )),
        }
    }

    pub fn frame(&mut self, time_seconds: f32) {
        #[cfg(feature = "audio")]
        self.context
            .audio
            .set_volume_setting(self.context.common_settings.volume);

        let elapsed_seconds = (time_seconds - self.context.client.time_seconds).clamp(0.001, 0.5);
        self.context.client.time_seconds = time_seconds;

        for inbound in self
            .context
            .socket
            .update(&mut self.context.state, time_seconds)
        {
            match &inbound {
                &Update::Client(ClientUpdate::SessionCreated {
                    arena_id,
                    cohort_id,
                    session_id,
                    server_id,
                    ..
                }) => {
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
                        .set_cohort_id(Some(cohort_id), &mut self.context.browser_storages);
                    self.context
                        .common_settings
                        .set_server_id(server_id, &mut self.context.browser_storages);

                    if self.context.socket.is_closed()
                        || Some((arena_id, session_id))
                            != self.context.common_settings.session_tuple()
                    {
                        self.context
                            .common_settings
                            .set_arena_id(Some(arena_id), &mut self.context.browser_storages);
                        self.context
                            .common_settings
                            .set_session_id(Some(session_id), &mut self.context.browser_storages);
                    }
                }
                Update::Client(ClientUpdate::EvalSnippet(snippet)) => {
                    // Do NOT use `eval`, since it runs in the local scope and therefore
                    // prevents minification.
                    let _ = Function::new_no_args(&snippet).call0(&JsValue::NULL);
                    // TODO: send result back to server.
                }
                _ => {}
            }

            if let Update::Game(update) = &inbound {
                self.game.peek_game(update, &mut self.context);
            }
            self.context.state.apply(inbound);
        }

        self.game.tick(elapsed_seconds, &mut self.context);

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
                        time: self.context.client.time_seconds,
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
        // these prevent chat from de-focusing:
        // event.prevent_default();
        // event.stop_propagation();
        let type_ = event.type_();

        match type_.as_str() {
            "mousedown" | "mouseup" => {
                if let Some(button) = MouseButton::try_from_button(event.button()) {
                    let down = type_ == "mousedown";

                    let e = GameClientMouseEvent::Button {
                        button,
                        down,
                        time: self.context.client.time_seconds,
                    };
                    self.game.peek_mouse(&e, &mut self.context);
                    self.context.mouse.apply(e);
                }
            }
            "mousemove" => {
                self.mouse_move_real(
                    event.client_x(),
                    event.client_y(),
                    event.movement_x(),
                    event.movement_y(),
                );
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
        event.prevent_default();
        event.stop_propagation();

        // Raw touch event.
        let touch_event = GameClientMouseEvent::Touch;
        self.game.peek_mouse(&touch_event, &mut self.context);
        self.context.mouse.apply(touch_event);

        // Don't care what event type, just consider the current set of touches.
        let target_touches = event.target_touches();

        let mut left_touch = None;
        let mut right_touch = None;

        for idx in 0..target_touches.length() {
            let touch: Touch = match target_touches.item(idx) {
                Some(touch) => touch,
                None => {
                    debug_assert!(false);
                    continue;
                }
            };

            let identifier = touch.identifier();
            if self.left_touch_id.map(|id| id == identifier).unwrap_or(
                self.right_touch_id
                    .map(|id| id != identifier)
                    .unwrap_or(true),
            ) {
                self.left_touch_id = Some(identifier);
                left_touch = Some(touch);
            } else if self
                .right_touch_id
                .map(|id| id == identifier)
                .unwrap_or(true)
            {
                self.right_touch_id = Some(identifier);
                right_touch = Some(touch);
            }
        }

        if let Some((first, second)) = left_touch
            .as_ref()
            .map(|t| IVec2::new(t.client_x(), t.client_y()).as_vec2())
            .zip(
                right_touch
                    .as_ref()
                    .map(|t| IVec2::new(t.client_x(), t.client_y()).as_vec2()),
            )
        {
            let pinch_distance = first.distance(second);

            if let Some(previous_pinch_distance) = self.context.mouse.pinch_distance {
                let delta = 0.03 * (previous_pinch_distance - pinch_distance);
                self.raw_zoom(delta);
            }

            self.context.mouse.pinch_distance = Some(pinch_distance);
        } else {
            self.context.mouse.pinch_distance = None;
        }

        macro_rules! process_touch {
            ($touch: expr, $mouse_button: expr, $overriden_by: expr, $overrides: expr, $id: ident) => {
                if let Some(touch) = $touch {
                    if self.context.mouse.is_down($mouse_button) {
                        self.mouse_move(touch.client_x(), touch.client_y());
                    } else {
                        if let Some(overrides) = $overrides {
                            if self.context.mouse.is_down(overrides) {
                                let e = GameClientMouseEvent::Button {
                                    button: overrides,
                                    down: false,
                                    time: self.context.client.time_seconds,
                                };
                                self.game.peek_mouse(&e, &mut self.context);
                                self.context.mouse.apply(e);
                            }
                        }

                        if $overriden_by
                            .map(|overriden_by| !self.context.mouse.is_down(overriden_by))
                            .unwrap_or(true)
                        {
                            self.mouse_move(touch.client_x(), touch.client_y());

                            // Start new click.
                            let e = GameClientMouseEvent::Button {
                                button: $mouse_button,
                                down: true,
                                time: self.context.client.time_seconds,
                            };
                            self.game.peek_mouse(&e, &mut self.context);
                            self.context.mouse.apply(e);
                        }
                    }
                } else {
                    self.$id = None;
                    if self.context.mouse.is_down($mouse_button) {
                        // Expire old click.
                        let e = GameClientMouseEvent::Button {
                            button: $mouse_button,
                            down: false,
                            time: self.context.client.time_seconds,
                        };
                        self.game.peek_mouse(&e, &mut self.context);
                        self.context.mouse.apply(e);
                    }
                }
            };
        }

        process_touch!(
            left_touch,
            MouseButton::Left,
            Some(MouseButton::Right),
            None,
            left_touch_id
        );
        process_touch!(
            right_touch,
            MouseButton::Right,
            None,
            Some(MouseButton::Left),
            right_touch_id
        );
    }

    /// For detecting when the browser tab becomes hidden.
    pub fn visibility_change(&mut self, _: Event) {
        // Written with the intention that errors bias towards visible=true.
        let visible = js_hooks::document().visibility_state() != web_sys::VisibilityState::Hidden;
        let e = VisibilityEvent::Visible(visible);
        self.game.peek_visibility(&e, &mut self.context);
        #[cfg(feature = "audio")]
        self.context.audio.peek_visibility(&e);
        self.context.visibility.apply(e)
    }

    /// Creates a mouse wheel event with the given delta.
    pub fn raw_zoom(&mut self, delta: f32) {
        let e = GameClientMouseEvent::Wheel(delta);
        self.game.peek_mouse(&e, &mut self.context);
        self.context.mouse.apply(e);
    }

    /// Converts page position (from event) to view position (-1..1).
    fn client_coordinate_to_view(x: i32, y: i32) -> Vec2 {
        let rect = js_hooks::canvas().get_bounding_client_rect();

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
        self.game.ui(event, &mut self.context);
    }

    /// Helper to issue a mouse move event from a real mouse event. Takes client coordinates.
    fn mouse_move_real(&mut self, x: i32, y: i32, dx: i32, dy: i32) {
        self.mouse_move(x, y);
        let e = GameClientMouseEvent::DeltaPixels(IVec2::new(dx, dy).as_vec2());
        self.game.peek_mouse(&e, &mut self.context);
        self.context.mouse.apply(e);
    }

    /// Helper to issue a mouse move event. Takes client coordinates.
    fn mouse_move(&mut self, x: i32, y: i32) {
        let view_position = Self::client_coordinate_to_view(x, y);

        let e = GameClientMouseEvent::MoveViewSpace(view_position);
        self.game.peek_mouse(&e, &mut self.context);
        self.context.mouse.apply(e);
    }

    pub fn wheel(&mut self, event: WheelEvent) {
        event.prevent_default();
        // each wheel step is 53 pixels.
        // do 0.5 or 1.0 raw zoom.
        let steps: f64 = event.delta_y() * (1.0 / 53.0);
        let sign = 1f64.copysign(steps);
        let steps = steps.abs().clamp(1.0, 2.0).floor() * sign;
        self.raw_zoom(steps as f32 * 0.5)
    }

    /// Sends any request to the server.
    pub fn send_request(&mut self, request: Request<G::GameRequest>) {
        self.context.socket.send(request);
    }

    /// Sends a command to the server to send a chat message.
    pub fn send_chat(&mut self, message: String, whisper: bool) {
        self.send_request(Request::Chat(ChatRequest::Send { message, whisper }));
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
            .set_protocol(protocol, &mut self.context.browser_storages);
    }

    /// Send error message to server.
    pub fn trace(&mut self, message: String) {
        self.context.send_trace(message);
    }

    /// Call when an advertisement was played.
    pub fn tally_ad(&mut self, ad_type: AdType) {
        self.context
            .send_to_server(Request::Client(ClientRequest::TallyAd(ad_type)));
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
            .set_server_id(server_id, &mut self.context.browser_storages);
    }

    /// Simulates dropping of one or both websockets.
    pub fn simulate_drop_web_socket(&mut self) {
        self.context.socket.simulate_drop();
    }
}
