// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use crate::component::context_menu::ContextMenuProps;
use crate::ptr_eq_rc::PtrEqRc;
use crate::Route;
use client_util::browser_storage::BrowserStorages;
use client_util::context::CoreState;
use client_util::frontend::Frontend;
use client_util::game_client::GameClient;
use client_util::setting::CommonSettings;
use core_protocol::id::{GameId, LanguageId, ServerId};
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{ChatRequest, PlayerRequest, SystemQuery, SystemResponse, TeamRequest};
use std::ops::Deref;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Request, RequestInit, RequestMode, Response, Url};
use yew::{use_context, Callback, Properties};

#[derive(Properties, PartialEq)]
pub struct PropertiesWrapper<P: PartialEq> {
    pub props: P,
}

impl<P: PartialEq> Deref for PropertiesWrapper<P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

/// Non-game-specific context wrapper.
#[derive(Clone, PartialEq)]
pub struct Ctw {
    pub game_id: GameId,
    pub setting_cache: SettingCache,
    pub change_common_settings_callback:
        Callback<Box<dyn FnOnce(&mut CommonSettings, &mut BrowserStorages)>>,
    pub chat_request_callback: Callback<ChatRequest>,
    pub context_menu: Option<ContextMenuProps>,
    pub player_request_callback: Callback<PlayerRequest>,
    pub raw_zoom_callback: Callback<f32>,
    pub set_context_menu_callback: Callback<Option<ContextMenuProps>>,
    /// A copy of the core state.
    pub state: PtrEqRc<CoreState>,
    pub team_request_callback: Callback<TeamRequest>,
}

impl Ctw {
    pub fn use_chat_request_callback() -> Callback<ChatRequest> {
        Self::use_ctw().chat_request_callback.clone()
    }

    pub fn use_player_request_callback() -> Callback<PlayerRequest> {
        Self::use_ctw().player_request_callback.clone()
    }

    pub fn use_change_common_settings_callback(
    ) -> Callback<Box<dyn FnOnce(&mut CommonSettings, &mut BrowserStorages)>> {
        Self::use_ctw().change_common_settings_callback.clone()
    }

    pub fn use_set_context_menu_callback() -> Callback<Option<ContextMenuProps>> {
        Self::use_ctw().set_context_menu_callback.clone()
    }

    pub fn use_core_state() -> PtrEqRc<CoreState> {
        Self::use_ctw().state
    }

    pub fn use_ctw() -> Self {
        use_context::<Self>().unwrap()
    }

    pub fn use_game_id() -> GameId {
        Self::use_ctw().game_id
    }

    pub fn use_raw_zoom_callback() -> Callback<f32> {
        Self::use_ctw().raw_zoom_callback.clone()
    }

    pub fn use_team_request_callback() -> Callback<TeamRequest> {
        Self::use_ctw().team_request_callback.clone()
    }
}

/// Game-specific context wrapper.
pub struct Gctw<G: GameClient> {
    pub send_ui_event_callback: Callback<G::UiEvent>,
}

impl<G: GameClient> Clone for Gctw<G> {
    fn clone(&self) -> Self {
        Self {
            send_ui_event_callback: self.send_ui_event_callback.clone(),
        }
    }
}

impl<G: GameClient> PartialEq for Gctw<G> {
    fn eq(&self, other: &Self) -> bool {
        self.send_ui_event_callback
            .eq(&other.send_ui_event_callback)
    }
}

impl<G: GameClient> Gctw<G> {
    pub fn send_ui_event(&self, ui_event: G::UiEvent) {
        Self::use_gctw().send_ui_event_callback.emit(ui_event);
    }

    /// Only works in function component.
    pub fn use_ui_event(ui_event: G::UiEvent) {
        Self::use_gctw().send_ui_event(ui_event);
    }

    /// Only works in function component.
    pub fn use_ui_event_callback() -> Callback<G::UiEvent> {
        Self::use_gctw().send_ui_event_callback.clone()
    }

    pub fn use_gctw() -> Self {
        use_context::<Self>().unwrap()
    }
}

pub struct Yew<P> {
    pub(crate) set_ui_props: Callback<P>,
    real_host: Option<String>,
    real_encryption: Option<bool>,
    ideal_server_id: Option<ServerId>,
}

impl<P: PartialEq> Yew<P> {
    pub(crate) async fn new(set_ui_props: Callback<P>) -> Self {
        Self::new_from_system(set_ui_props.clone())
            .await
            .map_err(|e| client_util::console_log!("system error: {}", e))
            .unwrap_or(Self {
                set_ui_props,
                real_host: None,
                real_encryption: None,
                ideal_server_id: None,
            })
    }

    async fn new_from_system(set_ui_props: Callback<P>) -> Result<Self, String> {
        use yew_router::Routable;

        let pathname = window()
            .unwrap()
            .location()
            .pathname()
            .map_err(|e| format!("{:?}", e))?;
        let invitation_id = Route::recognize(&pathname).and_then(|route| {
            if let Route::Invitation { invitation_id } = route {
                Some(invitation_id)
            } else {
                None
            }
        });

        let query = SystemQuery {
            // TODO: Hack.
            server_id: BrowserStorages::new().session.get("serverId", false),
            region_id: None,
            invitation_id,
        };

        let query_string = serde_urlencoded::to_string(&query).unwrap();

        let url = format!("/system/?{}", query_string);

        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);

        let request =
            Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{:?}", e))?;

        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| format!("{:?}", e))?;
        let resp: Response = resp_value.dyn_into().map_err(|e| format!("{:?}", e))?;
        let url = Url::new(&resp.url()).map_err(|e| format!("{:?}", e))?;
        let json_promise = resp.text().map_err(|e| format!("{:?}", e))?;
        let json: String = JsFuture::from(json_promise)
            .await
            .map_err(|e| format!("{:?}", e))?
            .as_string()
            .ok_or(String::from("JSON not string"))?;
        let decoded: SystemResponse = serde_json::from_str(&json).map_err(|e| e.to_string())?;

        Ok(Self {
            set_ui_props,
            real_host: Some(url.host()),
            real_encryption: Some(url.protocol() != "http:"),
            ideal_server_id: decoded.server_id,
        })
    }
}

impl<P: PartialEq> Frontend<P> for Yew<P> {
    fn set_ui_props(&self, props: P) {
        self.set_ui_props.emit(props);
    }

    fn get_real_host(&self) -> Option<String> {
        self.real_host.clone()
    }

    fn get_real_encryption(&self) -> Option<bool> {
        self.real_encryption
    }

    fn get_ideal_server_id(&self) -> Option<ServerId> {
        self.ideal_server_id
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct SettingCache {
    pub alias: Option<PlayerAlias>,
    pub chat_dialog_shown: bool,
    pub leaderboard_dialog_shown: bool,
    pub team_dialog_shown: bool,
    pub language_id: LanguageId,
    pub volume: f32,
}
