// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::instructions::Instructions;
use crate::ui::ship_menu::ShipMenu;
use crate::ui::{UiEvent, UiStatusPlaying};
use crate::Mk48Game;
use yew::{function_component, html, Html, Properties};
use yew_frontend::component::positioner::Position;
use yew_frontend::frontend::use_ui_event_callback;

#[derive(Properties, PartialEq)]
pub struct UpgradeOverlayProps {
    pub position: Position,
    pub score: u32,
    pub status: UiStatusPlaying,
}

#[function_component(UpgradeOverlay)]
pub fn upgrade_overlay(props: &UpgradeOverlayProps) -> Html {
    let onclick = use_ui_event_callback::<Mk48Game>().reform(UiEvent::Upgrade);
    html! {
        <ShipMenu
            entity={Some((props.status.entity_type, props.status.position))}
            score={props.score}
            position={props.position.clone()}
            {onclick}
        >
            <Instructions position={props.position} status={props.status.instruction_status}/>
        </ShipMenu>
    }
}
