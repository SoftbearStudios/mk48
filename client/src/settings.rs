// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use client_util::browser_storage::BrowserStorages;
use client_util::js_util::is_mobile;
use client_util::setting::Settings;
use std::str::FromStr;

/// Settings can be set via Javascript (see util/settings.js and page/Settings.svelte).
#[derive(Clone, Default, PartialEq, Settings)]
pub struct Mk48Settings {
    pub animations: bool,
    #[setting(no_store)]
    pub cinematic: bool,
    pub circle_hud: bool,
    pub dynamic_waves: bool,
    pub fps_shown: bool,
    pub shadows: ShadowSetting,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ShadowSetting {
    None,
    Hard,
    Soft,
}

impl Default for ShadowSetting {
    fn default() -> Self {
        if is_mobile() {
            // Shadows are broken on many mobile devices, including Finn's relatively recent Android phone.
            Self::None
        } else {
            // Default soft shadows because their shadow map can render up to 16x faster without animations.
            // The whole shadow map is rendered every frame so this is valuable.
            // The increased cost of 9x samples will only affect sprite shader mainly (lower pixel count).
            Self::Soft
        }
    }
}

// Shadow can't be option because then FromStr wouldn't work.
impl ShadowSetting {
    pub fn is_none(self) -> bool {
        self == Self::None
    }

    pub fn is_some(self) -> bool {
        !self.is_none()
    }

    pub fn shader_define(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Hard => "#define SHADOWS\n",
            Self::Soft => "#define SOFT_SHADOWS\n#define SHADOWS\n",
        }
    }
}

// TODO use strum to derive ToString and FromStr.
impl ToString for ShadowSetting {
    fn to_string(&self) -> String {
        match self {
            Self::None => "none",
            Self::Hard => "hard",
            Self::Soft => "soft",
        }
        .to_string()
    }
}

impl FromStr for ShadowSetting {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "none" => Self::None,
            "hard" => Self::Hard,
            "soft" => Self::Soft,
            _ => return Err(()),
        })
    }
}
