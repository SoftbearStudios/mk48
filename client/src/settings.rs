// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use client_util::local_storage::LocalStorage;
use client_util::setting::Settings;

/// Settings can be set via Javascript (see util/settings.js and page/Settings.svelte).
#[derive(Settings)]
pub struct Mk48Settings {
    pub(crate) animations: bool,
    #[setting(range = "0..3")]
    pub(crate) wave_quality: u8,
    #[setting(range = "0.0..1.0", finite)]
    pub(crate) volume: f32,
}

impl Default for Mk48Settings {
    fn default() -> Self {
        Self {
            animations: true,
            wave_quality: 1,
            volume: 0.5,
        }
    }
}
