// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::event::event_target;
use crate::frontend::{use_change_common_settings_callback, use_ctw};
use crate::translation::{use_translation, Translation};
use core_protocol::id::LanguageId;
use gloo::timers::callback::Timeout;
use stylist::yew::styled_component;
use web_sys::{Event, HtmlSelectElement};
use yew::{html, html_nested, use_state, Callback, Html};
use yew_icons::{Icon, IconId};

#[styled_component(LanguageMenu)]
pub fn language_menu() -> Html {
    let ctw = use_ctw();
    // Open if [`Some`], closed otherwise. The [`Some`] variant stores a timer to close it automatically.
    let menu_open = use_state::<Option<Timeout>, _>(|| None);

    let div_css_class = css!(
        r#"
        height: 2rem;
        position: relative;
        width: 2rem;
    "#
    );

    let select_css_class = css!(
        r#"
        background-color: #CCC;
        color: black;
        position: absolute;
        right: 0;
        top: 0;
        width: min-content;
        border-radius: 0.25em;
        box-sizing: border-box;
        cursor: pointer;
        font-size: 0.8rem;
        font-weight: bold;
        outline: 0;
        padding: 0.7em;
        pointer-events: all;
        white-space: nowrap;
        margin-top: 0.25em;
        border: 0;
    "#
    );

    let handle_open = {
        let menu_open = menu_open.clone();

        Callback::from(move |_| {
            if menu_open.is_none() {
                let menu_open_clone = menu_open.clone();

                menu_open.set(Some(Timeout::new(10000, move || {
                    menu_open_clone.set(None);
                })));
            };
        })
    };

    let handle_change = {
        let change_common_settings_callback = use_change_common_settings_callback();
        let menu_open = menu_open.clone();

        move |event: Event| {
            let select: HtmlSelectElement = event_target(&event);
            let value = select.value();
            // TODO: Very bad code. Probably should use settings object + serde.
            let parsed = LanguageId::iter().find(|l| format!("{:?}", l) == value);
            if let Some(parsed) = parsed {
                change_common_settings_callback.emit(Box::new(
                    move |common_settings, browser_storage| {
                        common_settings.set_language(parsed, browser_storage);
                    },
                ));
            }
            menu_open.set(None);
        }
    };

    let t = use_translation();

    html! {
        <div class={div_css_class}>
            if menu_open.is_some() {
                <select onchange={handle_change} class={select_css_class}>
                    {LanguageId::iter().map(|language_id| {
                        html_nested!{
                            <option
                                value={format!("{:?}", language_id)}
                                selected={language_id == ctw.setting_cache.language}
                            >{language_id.label()}</option>
                        }
                    }).collect::<Html>()}
                </select>
            } else {
                <Icon
                    icon_id={IconId::BootstrapGlobe2}
                    width={String::from("2rem")}
                    height={String::from("1.8rem")}
                    title={t.settings_language_hint()}
                    onclick={handle_open}
                    style={"cursor: pointer;"}
                />
            }
        </div>
    }
}
