use crate::browser_storage::BrowserStorages;
use core_protocol::id::{ArenaId, ServerId, SessionId};
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
    /// Last-used/chosen [`ServerId`].
    #[setting(volatile)]
    pub server_id: Option<ServerId>,
    /// Not manually set by the player.
    pub arena_id: Option<ArenaId>,
    /// Not manually set by the player. Not accessible via arbitrary getter/setter as doing so would
    /// pull BigUint64Array into the JS shim, invalidating compatibility with old devices.
    #[setting(no_serde_wasm_bindgen)]
    pub session_id: Option<SessionId>,
    /// Whether to set antialias rendering option.
    pub antialias: bool,
    /// Websocket protocol.
    #[setting(no_serde_wasm_bindgen, volatile)]
    pub protocol: WebSocketProtocol,
}

impl Default for CommonSettings {
    fn default() -> Self {
        Self {
            server_id: None,
            arena_id: None,
            session_id: None,
            antialias: true,
            protocol: WebSocketProtocol::default(),
        }
    }
}

impl CommonSettings {
    /// Gets the `ArenaId` and `SessionId` together, or `None` if either or both are missing.
    pub(crate) fn session_tuple(&self) -> Option<(ArenaId, SessionId)> {
        self.arena_id.zip(self.session_id)
    }
}
