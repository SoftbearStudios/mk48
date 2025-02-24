// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

pub(crate) mod about_dialog;
pub(crate) mod game_ui;
pub(crate) mod help_dialog;
pub(crate) mod hint;
pub(crate) mod logo;
mod phrases;
pub(crate) mod references_dialog;
pub(crate) mod respawn_overlay;
pub(crate) mod ship_menu;
pub(crate) mod ships_dialog;
pub(crate) mod sprite;
pub(crate) mod status_overlay;
pub(crate) mod team;
pub(crate) mod upgrade_overlay;

pub use game_ui::{
    Mk48Route, Mk48Ui, UiEvent, UiProps, UiState, UiStatus, UiStatusPlaying, UiStatusRespawning,
};
pub(crate) use phrases::Mk48Phrases;
