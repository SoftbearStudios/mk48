// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::ship_menu::ShipMenu;
use crate::ui::{Mk48Phrases, UiEvent, UiStatusRespawning};
use crate::Mk48Game;
use kodiak_client::{
    use_interstitial_ad, use_splash_screen, use_translator, use_ui_event_callback, InterstitialAd,
};
use stylist::yew::styled_component;
use yew::{html, Callback, Html, Properties};

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

    let t = use_translator();
    let (_paused, _transitioning, onanimationend) = use_splash_screen();
    let ui_event_callback = use_ui_event_callback::<Mk48Game>();
    let interstitial_ad = use_interstitial_ad();
    let onclick = Callback::from(move |entity_type| {
        let onspawn = ui_event_callback.reform(move |_| UiEvent::Respawn(entity_type));
        if let InterstitialAd::Available { request } = &interstitial_ad {
            request.emit(Some(onspawn));
        } else {
            onspawn.emit(());
        }
    });
    html! {
        <div id="death" class={container_style} {onanimationend}>
            <h2 class={reason_style}>{t.death_reason(&props.status.death_reason)}</h2>
            <ShipMenu
                score={props.score}
                {onclick}
                closable={false}
            />
            <div
                id="banner_container"
                data-instance="respawn"
                data-fallback="bottom"
                style="margin: 5rem auto;"
            ></div>
        </div>
    }
}
