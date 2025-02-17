// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use kodiak_client::{
    is_mobile, settings_prerequisites, BrowserStorages, SettingCategory, Settings, Translator,
};
use strum_macros::{EnumIter, EnumMessage, EnumString, IntoStaticStr};

/// Settings can be set via Javascript (see util/settings.js and page/Settings.svelte).
#[derive(Clone, PartialEq, Settings)]
pub struct Mk48Settings {
    #[setting(preference, checkbox = "Graphics/Antialias", post)]
    pub antialias: bool,
    #[setting(preference, checkbox = "Graphics/Animations", post)]
    pub animations: bool,
    #[setting(preference, checkbox = "Circle HUD", post)]
    pub circle_hud: bool,
    #[setting(preference, checkbox = "Graphics/Dynamic Waves", post)]
    pub dynamic_waves: bool,
    #[setting(preference, checkbox = "Show FPS Counter", post)]
    pub fps_shown: bool,
    /// Whether team menu is open.
    #[setting(preference, volatile, post)]
    pub team_dialog_shown: bool,
    #[setting(preference, dropdown = "Graphics/Shadows", post)]
    pub shadows: ShadowSetting,
}

impl Default for Mk48Settings {
    fn default() -> Self {
        Self {
            antialias: !is_mobile(),
            animations: false,
            circle_hud: false,
            dynamic_waves: false,
            fps_shown: false,
            team_dialog_shown: true,
            shadows: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, EnumString, EnumMessage, IntoStaticStr, EnumIter)]
pub enum ShadowSetting {
    #[strum(ascii_case_insensitive, message = "No shadows")]
    None,
    #[strum(ascii_case_insensitive, message = "Hard shadows")]
    Hard,
    #[strum(ascii_case_insensitive, message = "Soft shadows")]
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

impl ToString for ShadowSetting {
    fn to_string(&self) -> String {
        let str: &str = self.into();
        str.to_owned()
    }
}
