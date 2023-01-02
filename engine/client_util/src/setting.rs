// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::browser_storage::BrowserStorages;
use crate::js_util::is_mobile;
use core_protocol::id::{ArenaId, CohortId, LanguageId, ServerId, SessionId};
use core_protocol::name::PlayerAlias;
use core_protocol::web_socket::WebSocketProtocol;
pub use engine_macros::Settings;

/// Settings backed by local storage.
pub trait Settings: Sized {
    /// Loads all settings from local storage.
    fn load(l: &BrowserStorages, default: Self) -> Self;
}

// Useful if you don't want settings.
impl Settings for () {
    fn load(_: &BrowserStorages, _: Self) -> Self {}
}

/// Settings of the infrastructure, common to all games.
#[derive(Clone, PartialEq, Settings)]
pub struct CommonSettings {
    /// Alias preference.
    #[setting(optional)]
    pub alias: Option<PlayerAlias>,
    /// Language preference.
    pub language: LanguageId,
    /// Volume preference (0 to 1).
    #[setting(range = "0.0..1.0", finite)]
    pub volume: f32,
    /// Last [`CohortId`].
    #[setting(optional)]
    pub cohort_id: Option<CohortId>,
    /// Last-used/chosen [`ServerId`].
    #[setting(optional, volatile)]
    pub server_id: Option<ServerId>,
    /// Not manually set by the player.
    #[setting(optional)]
    pub arena_id: Option<ArenaId>,
    /// Not manually set by the player. Not accessible via arbitrary getter/setter as doing so would
    /// pull BigUint64Array into the JS shim, breaking compatibility with old devices.
    #[setting(optional)]
    pub session_id: Option<SessionId>,
    /// Whether to set antialias rendering option.
    pub antialias: bool,
    /// Websocket protocol.
    #[setting(volatile)]
    pub protocol: WebSocketProtocol,
    /// Pending chat message.
    #[setting(volatile)]
    pub chat_message: String,
    /// Whether to add a contrasting border behind UI elements.
    pub high_contrast: bool,
    /// Whether team menu is open.
    #[setting(volatile)]
    pub team_dialog_shown: bool,
    /// Whether chat menu is open.
    pub chat_dialog_shown: bool,
    /// Whether leaderboard menu is open.
    #[setting(volatile)]
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
            antialias: !is_mobile(),
            protocol: WebSocketProtocol::default(),
            chat_message: String::new(),
            high_contrast: false,
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
