// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use crate::component::context_menu::ContextMenuProps;
use crate::component::section::Section;
use crate::event::event_target;
use crate::translation::{t, Translation};
use crate::window::event_listener::WindowEventListener;
use crate::Ctw;
use core_protocol::rpc::{ChatRequest, PlayerRequest};
use std::ops::Deref;
use stylist::yew::styled_component;
use web_sys::{HtmlInputElement, InputEvent, KeyboardEvent, MouseEvent};
use yew::{html, html_nested, use_effect_with_deps, use_node_ref, use_state, Html, Properties};

#[derive(Default, PartialEq, Properties)]
pub struct ChatProps {
    #[prop_or_default]
    pub hints: Vec<(&'static str, Vec<&'static str>)>,
}

#[styled_component(ChatOverlay)]
pub fn chat_overlay(props: &ChatProps) -> Html {
    let context_menu_css_class = css!(
        r#"
        background-color: #444444aa;
        color: black;
        min-width: 100px;
        position: absolute;
        transform: translate(-50%, -50%);

        button {
            background-color: #444444aa;
            border: 0;
            border-radius: 0;
            color: white;
            outline: 0;
            margin: 0;
            padding: 5px;
        }

        button:hover {
            filter: brightness(2.0);
        }

        button:hover:active {
            filter: brightness(1.0);
        }
    "#
    );

    let message_css_class = css!(
        r#"
        color: white;
        margin-bottom: 0.25em;
		margin-top: 0.25em;
		overflow-wrap: anywhere;
		text-overflow: ellipsis;
		word-break: normal;
        "#
    );

    let name_css_class = css!(
        r#"
        cursor: pointer;
		font-weight: bold;
		white-space: nowrap;
    "#
    );

    let official_name_css_class = css!(
        r#"
        font-weight: bold;
		white-space: nowrap;
        color: #fffd2a;
		text-shadow: 0px 0px 3px #381616;
        "#
    );

    let input_css_class = css!(
        r#"
        border-radius: 0.25em;
        box-sizing: border-box;
        cursor: pointer;
        font-size: 1rem;
        font-weight: bold;
        outline: 0;
        padding: 0.5em;
        pointer-events: all;
        white-space: nowrap;
        margin-top: 0.25em;
        background-color: #00000025;
        border: 0;
        color: white;
        width: 100%;
        "#
    );

    let input_ref = use_node_ref();
    let message = use_state(String::new);

    let oninput = {
        let message = message.clone();

        move |event: InputEvent| {
            let input: HtmlInputElement = event_target(&event);
            message.set(input.value());
        }
    };

    const ENTER: u32 = 13;

    let onkeydown = {
        let message = message.clone();
        let chat_request_callback = Ctw::use_chat_request_callback();

        move |event: KeyboardEvent| {
            if event.key_code() != ENTER {
                return;
            }
            event.stop_propagation();
            let input: HtmlInputElement = event_target(&event);
            let _ = input.blur();
            if message.is_empty() {
                return;
            }
            chat_request_callback.emit(ChatRequest::Send {
                message: message.deref().clone(),
                whisper: event.shift_key(),
            });
            message.set(String::new());
        }
    };

    // Pressing Enter key focuses the input.
    {
        let input_ref = input_ref.clone();

        use_effect_with_deps(
            |input_ref| {
                let input_ref = input_ref.clone();

                let onkeydown = WindowEventListener::new(
                    "keydown",
                    move |e: &KeyboardEvent| {
                        if e.key_code() == ENTER {
                            match input_ref.cast::<HtmlInputElement>() {
                                Some(input) => {
                                    let _ = input.focus();
                                }
                                None => {
                                    // Most likely the chat was closed.
                                }
                            };
                        }
                    },
                    false,
                );

                move || std::mem::drop(onkeydown)
            },
            input_ref,
        );
    }

    let core_state = Ctw::use_core_state();

    let items = core_state.messages.iter().map(|dto| {

        let onclick = {
            let at_alias = format!("@{} ", dto.alias).to_string();
            let message = message.clone();
            move || {
                // Don't overwrite an unsent (not empty) message.
                if message.is_empty() {
                    message.set(at_alias.clone());
                }
            }
        };

        let oncontextmenu = if Ctw::use_ctw().context_menu.is_some() || dto.player_id.is_none() {
            None
        } else {
            let player_id = dto.player_id.unwrap();
            let chat_request_callback = Ctw::use_chat_request_callback();
            let context_menu_css_class = context_menu_css_class.clone();
            let player_request_callback = Ctw::use_player_request_callback();
            let set_context_menu_callback = Ctw::use_set_context_menu_callback();

            Some(move |e: MouseEvent| {
                e.prevent_default();
                e.stop_propagation();
                let chat_request_callback = chat_request_callback.clone();
                let context_menu_css_class = context_menu_css_class.clone();
                let player_request_callback = player_request_callback.clone();
                let set_context_menu_callback = set_context_menu_callback.clone();
                let onclick_mute = {
                    let chat_request_callback = chat_request_callback.clone();
                    let set_context_menu_callback = set_context_menu_callback.clone();
                    move |_: MouseEvent| {
                        client_util::console_log!("mute: {}", player_id.0);
                        chat_request_callback.emit(ChatRequest::Mute(player_id));
                        set_context_menu_callback.emit(None);
                    }
                };
                let onclick_report = {
                    let player_request_callback = player_request_callback.clone();
                    let set_context_menu_callback = set_context_menu_callback.clone();
                    move |_: MouseEvent| {
                        client_util::console_log!("report: {}", player_id.0);
                        player_request_callback.emit(PlayerRequest::Report(player_id));
                        set_context_menu_callback.emit(None);
                    }
                };
                let style = format!("left: {}px; top: {}px;", e.x(), e.y());

                let html = html!{
                    <div id="context_menu" class={context_menu_css_class} style={style}>
                        <button onclick={onclick_mute}>{"Mute player"}</button>
                        <button onclick={onclick_report}>{"Report player"}</button>
                    </div>
                };
                set_context_menu_callback.emit(Some(ContextMenuProps{html}));
            })
        };

        html_nested!{
            <p class={message_css_class.clone()} onclick={move |_| onclick()} oncontextmenu={oncontextmenu}>
                <span
                    class={if dto.player_id.is_some() { name_css_class.clone() } else { official_name_css_class.clone() }}
                >{dto.team_name.map(|team_name| format!("[{}] {}", team_name, dto.alias)).unwrap_or(dto.alias.to_string())}</span>{format!(" {}", dto.text)}
            </p>
        }
    }).collect::<Html>();

    let title = if core_state.team_id().is_some() {
        t().chat_send_team_message_hint()
    } else {
        t().chat_send_message_hint()
    };

    let help_hint = help_hint_of(props, message.deref());

    html! {
        <Section name={t().chat_label()}>
            {items}
            if let Some(help_hint) = help_hint {
                <p><b>{"Automated help: "}{help_hint}</b></p>
            }
            <input
                type="text"
                name="message"
                {title}
                {oninput}
                {onkeydown}
                autocomplete="off"
                minLength="1"
                maxLength="128"
                value={message.deref().clone()}
                placeholder={t().chat_send_message_placeholder()}
                class={input_css_class.clone()}
                ref={input_ref}
            />
        </Section>
    }
}

fn help_hint_of(props: &ChatProps, text: &str) -> Option<&'static str> {
    if text.find("/invite").is_some() {
        Some("Invitation links cannot currently be accepted by players that are already in game. They must send a join request instead.")
    } else {
        for (value, keys) in props.hints.iter() {
            let mut found = true;
            for k in keys.iter() {
                if !text.find(k).is_some() {
                    found = false;
                }
            }
            if found {
                return Some(value);
            }
        }

        None
    }
}
