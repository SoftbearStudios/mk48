// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::Route;
use client_util::browser_storage::BrowserStorages;
use client_util::context::{StrongCoreState, WeakCoreState};
use client_util::frontend::Frontend;
use client_util::game_client::GameClient;
use client_util::js_util::referrer;
use client_util::setting::CommonSettings;
use core_protocol::id::{GameId, ServerId};
use core_protocol::name::Referrer;
use core_protocol::rpc::{ChatRequest, PlayerRequest, SystemQuery, SystemResponse, TeamRequest};
use js_hooks::console_log;
use std::ops::Deref;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, Request, RequestInit, RequestMode, Response, Url};
use yew::{hook, use_context, Callback, Html, Properties};
use yew_router::Routable;

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

#[derive(Clone, PartialEq)]
pub enum RewardedAd {
    Unavailable,
    Available {
        /// Start watching.
        request: Callback<()>,
    },
    Watching,
    Watched {
        /// Set back to available.
        consume: Callback<()>,
    },
    Canceled,
}

/// Non-game-specific context wrapper.
#[derive(Clone, PartialEq)]
pub struct Ctw {
    pub game_id: GameId,
    /// Outbound links.
    pub outbound_enabled: bool,
    pub rewarded_ad: RewardedAd,
    pub setting_cache: CommonSettings,
    pub change_common_settings_callback:
        Callback<Box<dyn FnOnce(&mut CommonSettings, &mut BrowserStorages)>>,
    pub chat_request_callback: Callback<ChatRequest>,
    pub player_request_callback: Callback<PlayerRequest>,
    pub raw_zoom_callback: Callback<f32>,
    pub recreate_renderer_callback: Callback<()>,
    pub set_server_id_callback: Callback<Option<ServerId>>,
    pub set_context_menu_callback: Callback<Option<Html>>,
    pub(crate) routes: Vec<&'static str>,
    /// A copy of the core state.
    pub state: WeakCoreState,
    pub team_request_callback: Callback<TeamRequest>,
    pub licenses: &'static [(&'static str, &'static [&'static str])],
}

#[hook]
pub fn use_rewarded_ad() -> RewardedAd {
    use_ctw().rewarded_ad
}

#[hook]
pub fn use_chat_request_callback() -> Callback<ChatRequest> {
    use_ctw().chat_request_callback
}

#[hook]
pub fn use_player_request_callback() -> Callback<PlayerRequest> {
    use_ctw().player_request_callback
}

#[hook]
pub fn use_change_common_settings_callback(
) -> Callback<Box<dyn FnOnce(&mut CommonSettings, &mut BrowserStorages)>> {
    use_ctw().change_common_settings_callback
}

#[hook]
pub fn use_set_context_menu_callback() -> Callback<Option<Html>> {
    use_ctw().set_context_menu_callback
}

#[hook]
pub fn use_core_state() -> StrongCoreState<'static> {
    use_ctw().state.into_strong()
}

#[hook]
pub fn use_ctw() -> Ctw {
    use_context::<Ctw>().unwrap()
}

#[hook]
pub fn use_game_id() -> GameId {
    use_ctw().game_id
}

#[hook]
pub fn use_raw_zoom_callback() -> Callback<f32> {
    use_ctw().raw_zoom_callback
}

#[hook]
pub fn use_team_request_callback() -> Callback<TeamRequest> {
    use_ctw().team_request_callback
}

#[hook]
pub fn use_outbound_enabled() -> bool {
    use_ctw().outbound_enabled
}

/// Game-specific context wrapper.
pub struct Gctw<G: GameClient> {
    pub send_ui_event_callback: Callback<G::UiEvent>,
    pub change_settings_callback:
        Callback<Box<dyn FnOnce(&mut G::GameSettings, &mut BrowserStorages)>>,
    pub settings_cache: G::GameSettings,
}

impl<G: GameClient> Clone for Gctw<G> {
    fn clone(&self) -> Self {
        Self {
            send_ui_event_callback: self.send_ui_event_callback.clone(),
            change_settings_callback: self.change_settings_callback.clone(),
            settings_cache: self.settings_cache.clone(),
        }
    }
}

impl<G: GameClient> PartialEq for Gctw<G> {
    fn eq(&self, other: &Self) -> bool {
        self.send_ui_event_callback
            .eq(&other.send_ui_event_callback)
            && self
                .change_settings_callback
                .eq(&other.change_settings_callback)
            && self.settings_cache == other.settings_cache
    }
}

/// Only works in function component.
#[hook]
pub fn use_ui_event_callback<G: GameClient>() -> Callback<G::UiEvent> {
    use_gctw::<G>().send_ui_event_callback
}

#[hook]
pub fn use_change_settings_callback<G: GameClient>(
) -> Callback<Box<dyn FnOnce(&mut G::GameSettings, &mut BrowserStorages)>> {
    use_gctw::<G>().change_settings_callback
}

#[hook]
pub fn use_gctw<G: GameClient>() -> Gctw<G> {
    use_context::<Gctw<G>>().unwrap()
}

pub struct Yew<P> {
    set_ui_props: Callback<P>,
    referrer: Option<Referrer>,
    system_info: Option<SystemInfo>,
}

/// Information derived from a system request.
pub(crate) struct SystemInfo {
    host: String,
    encryption: bool,
    ideal_server_id: Option<ServerId>,
}

impl<P: PartialEq> Yew<P> {
    pub(crate) async fn new(set_ui_props: Callback<P>) -> Self {
        Self {
            set_ui_props,
            referrer: get_real_referrer(),
            system_info: SystemInfo::new()
                .await
                .inspect_err(|e| console_log!("system error: {}", e))
                .ok(),
        }
    }
}

impl SystemInfo {
    async fn new() -> Result<Self, String> {
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
            server_id: BrowserStorages::new().session.get("serverId"),
            region_id: None,
            invitation_id,
        };

        let query_string = serde_urlencoded::to_string(&query).unwrap();

        let url = format!("/system.json?{}", query_string);

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
            host: url.host(),
            encryption: url.protocol() != "http:",
            ideal_server_id: decoded.server_id,
        })
    }
}

impl<P: PartialEq> Frontend<P> for Yew<P> {
    fn set_ui_props(&self, props: P) {
        self.set_ui_props.emit(props);
    }

    fn get_real_referrer(&self) -> Option<Referrer> {
        self.referrer
    }

    fn get_real_host(&self) -> Option<String> {
        self.system_info.as_ref().map(|i| i.host.clone())
    }

    fn get_real_encryption(&self) -> Option<bool> {
        self.system_info.as_ref().map(|i| i.encryption)
    }

    fn get_ideal_server_id(&self) -> Option<ServerId> {
        self.system_info.as_ref().and_then(|i| i.ideal_server_id)
    }
}

fn get_real_referrer() -> Option<Referrer> {
    window()
        .unwrap()
        .location()
        .pathname()
        .ok()
        .and_then(|pathname| Route::recognize(&pathname))
        .and_then(|route| {
            if let Route::Referrer { referrer } = route {
                console_log!("overriding referrer to: {}", referrer);
                Some(referrer)
            } else {
                None
            }
        })
        .or_else(referrer)
}

/// Post message to window.
pub(crate) fn post_message(message: &str) {
    if window()
        .unwrap()
        .post_message(&JsValue::from_str(message), "*")
        .is_err()
    {
        console_log!("error posting message");
    }
}
