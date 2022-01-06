use crate::local_storage::LocalStorage;
use core_protocol::id::{ArenaId, SessionId};
pub use engine_macros::Settings;
use wasm_bindgen::JsValue;

/// Settings backed by local storage.
pub trait Settings: Default {
    /// Loads all settings from local storage.
    fn load(local_storage: &LocalStorage) -> Self;

    /// Gets an arbitrary setting as JS. Returns `JsValue::NULL` if setting is unrecognized, to allow for
    /// multiple instances in parallel.
    fn get(&self, key: &str) -> JsValue;

    /// Sets a arbitrary setting from JS. Does nothing if setting is unrecognized, to allow for
    /// multiple instances in parallel.
    fn set(&mut self, key: &str, value: JsValue, local_storage: &mut LocalStorage);
}

// Useful if you don't want settings.
impl Settings for () {
    fn load(_local_storage: &LocalStorage) -> Self {}
    fn get(&self, _key: &str) -> JsValue {
        JsValue::NULL
    }
    fn set(&mut self, _key: &str, _value: JsValue, _local_storage: &mut LocalStorage) {}
}

/// Settings of the infrastructure, common to all games.
#[derive(Settings)]
pub(crate) struct CommonSettings {
    /// Not manually set by the player.
    pub arena_id: Option<ArenaId>,
    /// Not manually set by the player.
    pub session_id: Option<SessionId>,
    /// Whether to set antialias rendering option.
    #[setting(default = "true")]
    pub antialias: bool,
}

impl CommonSettings {
    /// Gets the `ArenaId` and `SessionId` together, or `None` if either or both are missing.
    pub(crate) fn session_tuple(&self) -> Option<(ArenaId, SessionId)> {
        self.arena_id.zip(self.session_id)
    }
}
