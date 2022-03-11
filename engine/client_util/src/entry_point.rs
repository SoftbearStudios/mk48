// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

/// A game client should invoke this macro, passing the type implementing GameClient.
#[macro_export]
macro_rules! entry_point {
    ($G: ty) => {
        use core_protocol::id::{PlayerId, ServerId, TeamId};
        use core_protocol::name::TeamName;
        use serde::de::DeserializeOwned;
        use std::cell::{RefCell, RefMut, UnsafeCell};
        use std::mem::MaybeUninit;
        use std::num::NonZeroU32;
        use wasm_bindgen::prelude::*;
        use web_sys::{FocusEvent, KeyboardEvent, MouseEvent, TouchEvent, WheelEvent};

        pub static mut INFRASTRUCTURE: UnsafeCell<
            MaybeUninit<RefCell<client_util::infrastructure::Infrastructure<$G>>>,
        > = UnsafeCell::new(MaybeUninit::uninit());

        /// Easily get the infrastructure.
        fn borrow_infrastructure(
        ) -> RefMut<'static, client_util::infrastructure::Infrastructure<$G>> {
            unsafe { INFRASTRUCTURE.get_mut().assume_init_ref().borrow_mut() }
        }

        /// Easily access the infrastructure.
        ///
        /// This should be used if mitigation of JavaScript "Immediate Events"
        /// is a concern (if calls were observed to interrupt already-executing WebAssembly).
        fn with_infrastructure<
            F: FnOnce(RefMut<'static, client_util::infrastructure::Infrastructure<$G>>) -> R,
            R,
        >(
            function: F,
        ) -> Option<R> {
            unsafe {
                if let Ok(infrastructure) =
                    INFRASTRUCTURE.get_mut().assume_init_ref().try_borrow_mut()
                {
                    return Some(function(infrastructure));
                } else {
                    return None;
                }
            }
        }

        #[wasm_bindgen]
        pub fn frame(time_seconds: f32) {
            with_infrastructure(|mut i| i.frame(time_seconds));
        }

        #[wasm_bindgen]
        pub fn keyboard(event: KeyboardEvent) {
            with_infrastructure(move |mut i| i.keyboard(event));
        }

        #[wasm_bindgen(js_name = "keyboardFocus")]
        pub fn keyboard_focus(event: FocusEvent) {
            with_infrastructure(move |mut i| i.keyboard_focus(event));
        }

        #[wasm_bindgen]
        pub fn mouse(event: MouseEvent) {
            with_infrastructure(move |mut i| i.mouse(event));
        }

        #[wasm_bindgen(js_name = "mouseFocus")]
        pub fn mouse_focus(event: FocusEvent) {
            with_infrastructure(move |mut i| i.mouse_focus(event));
        }

        #[wasm_bindgen]
        pub fn touch(event: TouchEvent) {
            with_infrastructure(move |mut i| i.touch(event));
        }

        #[wasm_bindgen]
        pub fn wheel(event: WheelEvent) {
            with_infrastructure(move |mut i| i.wheel(event));
        }

        #[wasm_bindgen]
        pub fn zoom(amount: f32) {
            with_infrastructure(move |mut i| i.raw_zoom(amount));
        }

        #[wasm_bindgen]
        pub fn event(event: JsValue) {
            match serde_wasm_bindgen::from_value(event) {
                Ok(event) => {
                    with_infrastructure(move |mut i| i.ui_event(event));
                }
                Err(e) => client_util::console_log!("could not parse UI event: {:?}", e),
            }
        }

        #[wasm_bindgen(js_name = "handleSendChat")]
        pub fn handle_send_chat(message: String, team: bool) {
            with_infrastructure(move |mut i| i.send_chat(message, team));
        }

        #[wasm_bindgen(js_name = "handleCreateTeam")]
        pub fn handle_create_team(name: String) {
            with_infrastructure(move |mut i| i.create_team(TeamName::new(&name)));
        }

        #[wasm_bindgen(js_name = "handleRequestJoinTeam")]
        pub fn handle_request_join_team(team_id: u32) {
            if let Some(team_id) = NonZeroU32::new(team_id).map(TeamId) {
                with_infrastructure(move |mut i| i.request_join_team(team_id));
            }
        }

        #[wasm_bindgen(js_name = "handleAcceptJoinTeam")]
        pub fn handle_accept_join_team(player_id: u32) {
            if let Some(player_id) = NonZeroU32::new(player_id).map(PlayerId) {
                with_infrastructure(move |mut i| i.accept_join_team(player_id));
            }
        }

        #[wasm_bindgen(js_name = "handleRejectJoinTeam")]
        pub fn handle_reject_join_team(player_id: u32) {
            if let Some(player_id) = NonZeroU32::new(player_id).map(PlayerId) {
                with_infrastructure(move |mut i| i.reject_join_team(player_id));
            }
        }

        #[wasm_bindgen(js_name = "handleKickFromTeam")]
        pub fn handle_kick_from_team(player_id: u32) {
            if let Some(player_id) = NonZeroU32::new(player_id).map(PlayerId) {
                with_infrastructure(move |mut i| i.kick_from_team(player_id));
            }
        }

        #[wasm_bindgen(js_name = "handleLeaveTeam")]
        pub fn handle_leave_team() {
            with_infrastructure(move |mut i| i.leave_team());
        }

        #[wasm_bindgen(js_name = "handleReportPlayer")]
        pub fn handle_report_player(player_id: u32) {
            if let Some(player_id) = NonZeroU32::new(player_id).map(PlayerId) {
                with_infrastructure(move |mut i| i.report_player(player_id));
            }
        }

        #[wasm_bindgen(js_name = "handleMutePlayer")]
        pub fn handle_mute_player(player_id: u32, mute: bool) {
            if let Some(player_id) = NonZeroU32::new(player_id).map(PlayerId) {
                with_infrastructure(move |mut i| i.mute_player(player_id, mute));
            }
        }

        #[wasm_bindgen(js_name = "handleWebSocketProtocol")]
        pub fn handle_web_socket_protocol(protocol: String) {
            with_infrastructure(move |mut i| i.web_socket_protocol(parse_enum(&protocol)));
        }

        #[wasm_bindgen(js_name = "handleTrace")]
        pub fn handle_trace(message: String) {
            with_infrastructure(move |mut i| i.trace(message));
        }

        #[wasm_bindgen(js_name = "getSetting")]
        pub fn get_setting(key: String) -> JsValue {
            with_infrastructure(move |mut i| i.get_setting(&key)).unwrap_or(JsValue::NULL)
        }

        #[wasm_bindgen(js_name = "setSetting")]
        pub fn set_setting(key: String, value: JsValue) {
            with_infrastructure(move |mut i| i.set_setting(&key, value));
        }

        #[wasm_bindgen(js_name = "handleChooseServerId")]
        pub fn handle_choose_server_id(server_id: Option<u8>) {
            let server_id = server_id.and_then(|server_id| ServerId::new(server_id));
            with_infrastructure(move |mut i| i.choose_server_id(server_id));
        }

        #[wasm_bindgen(js_name = "simulateDropWebSocket")]
        pub fn simulate_drop_web_socket() {
            with_infrastructure(move |mut i| i.simulate_drop_web_socket());
        }

        /// parse_enum deserializes a string into an enum, panicking if it doesn't match any variant.
        fn parse_enum<E: DeserializeOwned>(string: &str) -> E {
            let fmt = format!("\"{}\"", string);
            serde_json::from_str(&fmt).unwrap()
        }

        #[wasm_bindgen(start)]
        pub fn start() -> Result<(), wasm_bindgen::JsValue> {
            use client_util::game_client::GameClient;

            let infrastructure = RefCell::new(client_util::infrastructure::Infrastructure::new(
                <$G>::new(),
                Box::new(client_util::frontend::Svelte),
            ));

            unsafe {
                // SAFETY: This is the very first thing to run.
                *INFRASTRUCTURE.get_mut() = MaybeUninit::new(infrastructure);
            }
            Ok(())
        }
    };
}
