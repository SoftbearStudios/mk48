// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use client_util::local_storage::LocalStorage;
use client_util::setting::Settings;

/// Settings can be set via Javascript (see util/settings.js and page/Settings.svelte).
#[derive(Settings)]
pub struct Mk48Settings {
    #[setting(default = "true")]
    pub(crate) render_terrain_textures: bool,
    #[setting(default = "1", range = "0..3")]
    pub(crate) wave_quality: u8,
    #[setting(default = "0.5", range = "0.0..1.0")]
    pub(crate) volume: f32,
}
