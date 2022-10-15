// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::instructions::Instructions;
use crate::ui::ship_menu::ShipMenu;
use crate::ui::{UiEvent, UiStatusPlaying};
use crate::Mk48Game;
use yew::{function_component, html, Properties};
use yew_frontend::frontend::Gctw;

#[derive(Properties, PartialEq)]
pub struct UpgradeOverlayProps {
    pub score: u32,
    pub status: UiStatusPlaying,
}

#[function_component(UpgradeOverlay)]
pub fn upgrade_overlay(props: &UpgradeOverlayProps) -> Html {
    let onclick = Gctw::<Mk48Game>::use_ui_event_callback().reform(UiEvent::Upgrade);
    html! {
        <ShipMenu
            entity={Some((props.status.entity_type, props.status.position))}
            score={props.score}
            {onclick}
        >
            <Instructions ..props.status.instruction_props/>
        </ShipMenu>
    }
}
