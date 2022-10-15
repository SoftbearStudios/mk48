// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::positioner::Position;
use crate::frontend::post_message;
use crate::frontend::Ctw;
use crate::translation::{t, Translation};
use crate::WindowEventListener;
use core_protocol::name::PlayerAlias;
use gloo::timers::callback::Timeout;
use stylist::yew::styled_component;
use web_sys::{AnimationEvent, HtmlInputElement, MessageEvent};
use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct DialogProps {
    pub on_play: Callback<PlayerAlias>,
    #[prop_or(Position::Center)]
    pub position: Position,
    pub children: Children,
}

#[styled_component(SpawnOverlay)]
pub fn spawn_overlay(props: &DialogProps) -> Html {
    let form_style = css!(
        r#"
        display: flex;
        flex-direction: column;
        position: absolute;
        row-gap: 2rem;
        user-select: none;
        min-width: 50%;
        animation: fadein 1s;

        @keyframes fadein {
            from { opacity: 0; }
            to   { opacity: 1; }
        }
    "#
    );

    let input_style = css!(
        r#"
        background-color: #22222288;
        border-radius: 3rem;
        border: 0;
        box-sizing: border-box;
        color: #FFFA;
        cursor: pointer;
        font-size: 1.5rem;
        font-weight: bold;
        margin-top: 0.25em;
        outline: 0;
        padding-left: 2rem;
        padding: 0.7em;
        pointer-events: all;
        text-align: center;
        white-space: nowrap;
        width: 100%;
   "#
    );

    let button_style = css!(
        r#"
        background-color: #549f57;
        border-radius: 1rem;
        border: 1px solid #61b365;
        box-sizing: border-box;
        color: white;
        cursor: pointer;
        font-size: 3rem;
        left: 50%;
        margin-top: 0.5em;
        min-width: 12rem;
        padding-bottom: 0.7rem;
        padding-top: 0.5rem;
        position: relative;
        text-decoration: none;
        transform: translate(-50%, 0%);
        white-space: nowrap;
        width: min-content;

        :disabled {
            filter: brightness(0.8);
            cursor: initial;
        }

        :hover:not(:disabled) {
            filter: brightness(0.95);
        }

        :active:not(:disabled) {
            filter: brightness(0.9);
        }
    "#
    );

    let (paused, transitioning, onanimationend) = use_splash_screen();

    let alias_setting = Ctw::use_ctw().setting_cache.alias;
    let alias = use_state(|| alias_setting.unwrap_or(PlayerAlias::new_unsanitized("")));

    let oninput = {
        let alias = alias.clone();
        Callback::from(move |event: InputEvent| {
            alias.set(PlayerAlias::new_input_sanitized(
                &event.target_unchecked_into::<HtmlInputElement>().value(),
            ))
        })
    };

    let onplay = {
        let alias = alias.clone();
        let setting_callback = Ctw::use_change_common_settings_callback();
        props.on_play.reform(move |_| {
            let alias = *alias;
            setting_callback.emit(Box::new(move |settings, storages| {
                settings.set_alias(Some(alias), storages);
            }));
            alias
        })
    };

    let onclick = onplay.reform(|_: MouseEvent| {});

    // [`FocusEvent`] instead of [`SubmitEvent`] due to:
    // - https://github.com/rustwasm/wasm-bindgen/issues/2712
    // - https://github.com/yewstack/yew/issues/1359
    let onsubmit = onplay.reform(|event: FocusEvent| {
        event.prevent_default();
    });

    html! {
        <form id="spawn_overlay" class={form_style} style={props.position.to_string()} {onsubmit} {onanimationend}>
            {props.children.clone()}
            <input id="alias_input" class={input_style} disabled={*transitioning} type="text" name="name" placeholder={t().splash_screen_alias_placeholder()} autocomplete="off" value={alias.to_string()} {oninput}/>
            <button id="play_button" class={button_style} disabled={*paused || *transitioning} {onclick}>{t().splash_screen_play_label()}</button>
            <div id="banner_bottom" style="margin: auto;"></div>
        </form>
    }
}

/// Should be called on game-specific respawn screens.
pub fn use_splash_screen() -> (
    UseStateHandle<bool>,
    UseStateHandle<bool>,
    Option<Callback<AnimationEvent>>,
) {
    let paused = use_state(|| false);
    let transitioning = use_state(|| true);

    let onanimationend = transitioning.then(|| {
        let transitioning = transitioning.clone();
        Callback::from(move |_| {
            post_message("splash");
            transitioning.set(false);
        })
    });

    {
        let paused = paused.clone();
        let transitioning = transitioning.clone();

        // See https://yew.rs/docs/concepts/function-components/pre-defined-hooks for why dep is
        // needed.
        let transitioning_dep = *transitioning;

        use_effect_with_deps(
            |currently_transitioning| {
                let not_transitioning = !*currently_transitioning;
                let listener = WindowEventListener::new(
                    "message",
                    move |event: &MessageEvent| {
                        if let Some(message) = event.data().as_string() {
                            match message.as_str() {
                                "pause" => paused.set(true),
                                "unpause" => paused.set(false),
                                "snippetLoaded" if not_transitioning => post_message("splash"),
                                _ => {}
                            }
                        }
                    },
                    false,
                );

                // Defend against css animation end event not firing.
                let transition_timeout = not_transitioning
                    .then_some(Timeout::new(1500, move || transitioning.set(false)));

                || {
                    post_message("playing");
                    drop(listener);
                    drop(transition_timeout);
                }
            },
            transitioning_dep,
        );
    }

    (paused, transitioning, onanimationend)
}
