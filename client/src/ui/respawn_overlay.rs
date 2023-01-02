// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::translation::Mk48Translation;
use crate::ui::ship_menu::ShipMenu;
use crate::ui::{UiEvent, UiStatusRespawning};
use crate::Mk48Game;
use stylist::yew::styled_component;
use yew::{html, Html, Properties};
use yew_frontend::frontend::use_ui_event_callback;
use yew_frontend::overlay::spawn::use_splash_screen;
use yew_frontend::translation::use_translation;

#[derive(Properties, PartialEq)]
pub struct RespawnOverlayProps {
    pub score: u32,
    pub status: UiStatusRespawning,
}

#[styled_component(RespawnOverlay)]
pub fn respawn_overlay(props: &RespawnOverlayProps) -> Html {
    let container_style = css!(
        r#"
        left: 50%;
		min-width: 30%;
		padding-top: 1rem;
		position: absolute;
		top: 5%;
		transform: translate(-50%, 0);
		width: min-content;
		text-align: center;
		animation: fadein 1s;

        @keyframes fadein {
            from { opacity: 0; }
            to   { opacity: 1; }
        }
    "#
    );

    let reason_style = css!(
        r#"
        color: white;
		font-weight: bold;
		margin: 0;
		text-align: center;
		transition: filter 0.1s;
		user-select: none;
        "#
    );

    let t = use_translation();
    let (_paused, _transitioning, onanimationend) = use_splash_screen();
    let onclick = use_ui_event_callback::<Mk48Game>().reform(UiEvent::Respawn);
    html! {
        <div id="death" class={container_style} {onanimationend}>
            <h2 class={reason_style}>{t.death_reason(&props.status.death_reason)}</h2>
            <ShipMenu
                score={props.score}
                {onclick}
                closable={false}
            />
            <div id="banner_bottom" style="margin: 5rem auto;"></div>
        </div>
    }
}
