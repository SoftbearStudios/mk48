use crate::browser_storage::BrowserStorages;
use core_protocol::id::{ArenaId, CohortId, LanguageId, ServerId, SessionId};
use core_protocol::name::PlayerAlias;
use core_protocol::web_socket::WebSocketProtocol;
pub use engine_macros::Settings;
use wasm_bindgen::JsValue;

/// Settings backed by local storage.
pub trait Settings: Sized {
    /// Loads all settings from local storage.
    fn load(l: &BrowserStorages, default: Self) -> Self;

    /// Gets an arbitrary setting as JS. Returns `JsValue::NULL` if setting is unrecognized, to allow for
    /// multiple instances in parallel.
    fn get(&self, key: &str) -> JsValue;

    /// Sets a arbitrary setting from JS. Does nothing if setting is unrecognized, to allow for
    /// multiple instances in parallel.
    fn set(&mut self, key: &str, value: JsValue, l: &mut BrowserStorages);
}

// Useful if you don't want settings.
impl Settings for () {
    fn load(_: &BrowserStorages, _: Self) -> Self {}
    fn get(&self, _: &str) -> JsValue {
        JsValue::NULL
    }
    fn set(&mut self, _: &str, _: JsValue, _: &mut BrowserStorages) {}
}

/// Settings of the infrastructure, common to all games.
#[derive(Settings)]
pub struct CommonSettings {
    /// Alias preference.
    #[setting(unquote)]
    pub alias: Option<PlayerAlias>,
    /// Language preference.
    #[setting(unquote)]
    pub language: LanguageId,
    /// Volume preference (0 to 1).
    #[setting(range = "0.0..1.0", finite)]
    pub volume: f32,
    /// Last [`CohortId`].
    pub cohort_id: Option<CohortId>,
    /// Last-used/chosen [`ServerId`].
    #[setting(volatile)]
    pub server_id: Option<ServerId>,
    /// Not manually set by the player.
    pub arena_id: Option<ArenaId>,
    /// Not manually set by the player. Not accessible via arbitrary getter/setter as doing so would
    /// pull BigUint64Array into the JS shim, breaking compatibility with old devices.
    #[setting(no_serde_wasm_bindgen)]
    pub session_id: Option<SessionId>,
    /// Whether to set antialias rendering option.
    pub antialias: bool,
    /// Websocket protocol.
    #[setting(no_serde_wasm_bindgen, unquote, volatile)]
    pub protocol: WebSocketProtocol,
    /// Whether team menu is open.
    pub team_dialog_shown: bool,
    /// Whether chat menu is open.
    pub chat_dialog_shown: bool,
    /// Whether leaderboard menu is open.
    pub leaderboard_dialog_shown: bool,
}

impl Default for CommonSettings {
    fn default() -> Self {
        Self {
            alias: None,
            language: LanguageId::default(),
            volume: 0.5,
            cohort_id: None,
            server_id: None,
            arena_id: None,
            session_id: None,
            antialias: true,
            protocol: WebSocketProtocol::default(),
            team_dialog_shown: true,
            chat_dialog_shown: true,
            leaderboard_dialog_shown: true,
        }
    }
}

impl CommonSettings {
    /// Gets the `ArenaId` and `SessionId` together, or `None` if either or both are missing.
    pub(crate) fn session_tuple(&self) -> Option<(ArenaId, SessionId)> {
        self.arena_id.zip(self.session_id)
    }
}
