// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::context_menu::{ContextMenu, ContextMenuButton};
use crate::component::section::Section;
use crate::event::event_target;
use crate::translation::{t, Translation};
use crate::window::event_listener::WindowEventListener;
use crate::Ctw;
use client_util::browser_storage::BrowserStorages;
use client_util::setting::CommonSettings;
use core_protocol::id::LanguageId;
use core_protocol::rpc::{ChatRequest, PlayerRequest};
use std::ops::Deref;
use std::str::pattern::Pattern;
use stylist::yew::styled_component;
use web_sys::{window, HtmlInputElement, InputEvent, KeyboardEvent, MouseEvent};
use yew::{
    classes, html, html_nested, use_effect_with_deps, use_node_ref, use_state, Callback, Html,
    Properties,
};

#[derive(PartialEq, Properties)]
pub struct ChatProps {
    /// Override the default label.
    #[prop_or(LanguageId::chat_label)]
    pub label: fn(LanguageId) -> &'static str,
    #[prop_or_default]
    pub hints: Vec<(&'static str, Vec<&'static str>)>,
}

#[styled_component(ChatOverlay)]
pub fn chat_overlay(props: &ChatProps) -> Html {
    let message_css_class = css!(
        r#"
        color: white;
        margin-bottom: 0.25em;
		margin-top: 0.25em;
		overflow-wrap: anywhere;
		text-overflow: ellipsis;
		word-break: normal;
		user-select: text;
        "#
    );

    let name_css_class = css!(
        r#"
        cursor: pointer;
		font-weight: bold;
		white-space: nowrap;
		user-select: none;
    "#
    );

    let official_name_css_class = css!(
        r#"
        font-weight: bold;
		white-space: nowrap;
        color: #fffd2a;
		text-shadow: 0px 0px 3px #381616;
		user-select: none;
        "#
    );

    let no_select_style = css!(
        r#"
        user-select: none;
        "#
    );

    let mention_style = css!(
        r#"
        color: #cae3ec;
        font-weight: bold;
        background: #63ccee3d;
        border-radius: 0.25rem;
        padding: 0.1rem 0.15rem;
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

    let ctw = Ctw::use_ctw();

    let on_open_changed = ctw.change_common_settings_callback.reform(|open| {
        Box::new(
            move |common_settings: &mut CommonSettings, browser_storages: &mut BrowserStorages| {
                common_settings.set_chat_dialog_shown(open, browser_storages);
            },
        )
    });

    let t = t();
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
        let chat_request_callback = ctw.chat_request_callback;

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
    let (mention_string, moderator) = core_state
        .player()
        .map(|p| (format!("@{}", p.alias), p.moderator))
        .unwrap_or((String::from("PLACEHOLDER"), false));

    let items = core_state.messages.oldest_ordered().map(|dto| {
        let onclick_reply = {
            let at_alias = format!("@{} ", dto.alias).to_string();
            let message = message.clone();
            move || {
                // Don't overwrite an unsent (not empty) message.
                if message.is_empty() {
                    message.set(at_alias.clone());
                }
            }
        };

        let is_me = dto.player_id == core_state.player_id;
        let oncontextmenu = if let Some(player_id) = dto.player_id.filter(|_| moderator || !is_me) {
            let team_id = core_state.player_or_bot(player_id).and_then(|p| p.team_id);
            let chat_request_callback = Ctw::use_chat_request_callback();
            let player_request_callback = Ctw::use_player_request_callback();
            let set_context_menu_callback = Ctw::use_set_context_menu_callback();

            Some(move |e: MouseEvent| {
                e.prevent_default();
                e.stop_propagation();
                let chat_request_callback = chat_request_callback.clone();
                let player_request_callback = player_request_callback.clone();
                let onclick_mute = {
                    let chat_request_callback = chat_request_callback.clone();
                    Callback::from(move |_: MouseEvent| {
                        chat_request_callback.emit(ChatRequest::Mute(player_id));
                    })
                };
                let onclick_report = {
                    let player_request_callback = player_request_callback.clone();
                    Callback::from(move |_: MouseEvent| {
                        player_request_callback.emit(PlayerRequest::Report(player_id));
                    })
                };
                let onclick_restrict_5m = {
                    let chat_request_callback = chat_request_callback.clone();
                    Callback::from(move |_: MouseEvent| {
                        chat_request_callback.emit(ChatRequest::RestrictPlayer{player_id, minutes: 5 });
                    })
                };
                let onclick_copy_player_id = Callback::from(move |_: MouseEvent| {
                    if let Some(clipboard) = window().unwrap().navigator().clipboard() {
                        let _ = clipboard.write_text(&format!("{}", player_id.0));
                    }
                });
                let onclick_copy_team_id = team_id.map(|team_id| Callback::from(move |_: MouseEvent| {
                    if let Some(clipboard) = window().unwrap().navigator().clipboard() {
                        let _ = clipboard.write_text(&format!("{}", team_id.0));
                    }
                }));

                let html = html!{
                    <ContextMenu event={e}>
                        if moderator {
                            if !is_me {
                                <ContextMenuButton onclick={onclick_restrict_5m}>{"Restrict (5m)"}</ContextMenuButton>
                            }
                            <ContextMenuButton onclick={onclick_copy_player_id}>{"Copy ID"}</ContextMenuButton>
                            if let Some(onclick_copy_team_id) = onclick_copy_team_id {
                                 <ContextMenuButton onclick={onclick_copy_team_id}>{"Copy Team ID"}</ContextMenuButton>
                            }
                        } else {
                            <ContextMenuButton onclick={onclick_mute.clone()}>{t.chat_mute_label()}</ContextMenuButton>
                            <ContextMenuButton onclick={onclick_report}>{t.chat_report_label()}</ContextMenuButton>
                        }
                    </ContextMenu>
                };
                set_context_menu_callback.emit(Some(html));
            })
        } else {
            None
        };

        html_nested!{
            <p class={message_css_class.clone()} oncontextmenu={oncontextmenu}>
                <span
                    onclick={move |_| onclick_reply()}
                    class={if dto.player_id.is_some() { name_css_class.clone() } else { official_name_css_class.clone() }}
                >
                    {dto.team_name.map(|team_name| format!("[{}] {}", team_name, dto.alias)).unwrap_or(dto.alias.to_string())}
                </span>
                <span class={no_select_style.clone()}>{" "}</span>
                {segments(&dto.text, &mention_string).map(|Segment{contents, mention}| html_nested!{
                    <span class={classes!(mention.then(|| mention_style.clone()))}>{contents.to_owned()}</span>
                }).collect::<Html>()}
            </p>
        }
    }).collect::<Html>();

    let title = if core_state.team_id().is_some() {
        t.chat_send_team_message_hint()
    } else {
        t.chat_send_message_hint()
    };

    let help_hint = help_hint_of(props, message.deref());

    html! {
        <Section name={(props.label)(t)} open={ctw.setting_cache.chat_dialog_shown} {on_open_changed}>
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
                placeholder={t.chat_send_message_placeholder()}
                class={input_css_class.clone()}
                ref={input_ref}
            />
        </Section>
    }
}

fn help_hint_of(props: &ChatProps, text: &str) -> Option<&'static str> {
    let text = text.to_ascii_lowercase();
    if text.find("/invite").is_some() {
        Some("Invitation links cannot currently be accepted by players that are already in game. They must send a join request instead.")
    } else {
        for (value, keys) in props.hints.iter() {
            let mut found = true;
            for &k in keys.iter() {
                debug_assert_eq!(k, k.to_lowercase());
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

#[derive(Debug)]
struct Segment<'a> {
    pub contents: &'a str,
    pub mention: bool,
}

fn segments<'a, P: Pattern<'a> + Clone>(message: &'a str, mention: P) -> Segments<'a, P> {
    Segments { message, mention }
}

struct Segments<'a, P: Pattern<'a> + Clone> {
    message: &'a str,
    mention: P,
}

impl<'a, P: Pattern<'a> + Clone> Iterator for Segments<'a, P> {
    type Item = Segment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.message.is_empty() {
            // We are done.
            None
        } else {
            let (idx, mtch) = self
                .message
                .match_indices(self.mention.clone())
                .next()
                .unwrap_or((self.message.len(), self.message));
            if idx == 0 {
                // Mention is at the beginning, return it.
                let (before, after) = self.message.split_at(mtch.len());
                if before.is_empty() {
                    // Guard against empty pattern.
                    self.message = "";
                    return Some(Segment {
                        contents: after,
                        mention: false,
                    });
                }
                self.message = after;
                Some(Segment {
                    contents: before,
                    mention: true,
                })
            } else {
                // Mention is later on, return the non-mention before it.
                let (before, after) = self.message.split_at(idx);
                self.message = after;
                Some(Segment {
                    contents: before,
                    mention: false,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::overlay::chat::{segments, Segment};
    use rand::prelude::SliceRandom;
    use rand::{thread_rng, Rng};

    #[test]
    fn fuzz_segments() {
        fn random_string() -> String {
            std::iter::from_fn(|| ['a', '大', 'π'].choose(&mut thread_rng()))
                .take(thread_rng().gen_range(0..=12))
                .collect()
        }

        for _ in 0..200000 {
            let message = random_string();
            let mention = random_string();

            // Make sure it terminates, conserves characters, and doesn't return empty contents or
            // repeat non-mentions.
            let mut total = 0;
            let mut mentioned = true;
            for Segment { contents, mention } in segments(&message, &mention) {
                debug_assert!(!contents.is_empty());
                total += contents.len();
                if mention {
                    mentioned = true;
                } else {
                    debug_assert!(mentioned);
                    mentioned = false;
                }
            }
            debug_assert_eq!(message.len(), total);
        }
    }
}
