// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use crate::event::event_target;
use crate::svg::bork_flag::*;
use crate::translation::{t, Translation};
use crate::Ctw;
use core_protocol::id::LanguageId;
use gloo::timers::callback::Timeout;
use stylist::yew::styled_component;
use web_sys::{Event, HtmlSelectElement};
use yew::{html, html_nested, use_state, Html};
use yew_icons::{Icon, IconId};

#[styled_component(LanguageMenu)]
pub fn language_menu() -> Html {
    let ctw = Ctw::use_ctw();
    // Open if [`Some`], closed otherwise.
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

        move |_| {
            if menu_open.is_none() {
                let menu_open_clone = menu_open.clone();

                menu_open.set(Some(Timeout::new(10000, move || {
                    menu_open_clone.set(None);
                })));
            };
        }
    };

    let handle_change = {
        let change_common_settings_callback = Ctw::use_change_common_settings_callback();
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

    // <select value={language_id} onchange={|e| handle_change(e.target.value)} onclick={|e| e.stop_propagation()}>
    html! {
        <div id="language_selector" onclick={handle_open} class={div_css_class}>
            if menu_open.is_some() {
                <select onchange={handle_change} class={select_css_class}>
                    {LanguageId::iter().map(|language_id| {
                        html_nested!{
                            <option
                                value={format!("{:?}", language_id)}
                                selected={language_id == ctw.setting_cache.language_id}
                            >{language_id.label()}</option>
                        }
                    }).collect::<Html>()}
                </select>
            } else if ctw.setting_cache.language_id == LanguageId::Bork {
                <BorkFlag/>
            } else {
                <Icon icon_id={match ctw.setting_cache.language_id {
                        LanguageId::Bork => unreachable!(),
                        LanguageId::English => IconId::LipisFlagIcons4X3Gb,
                        LanguageId::German => IconId::LipisFlagIcons4X3De,
                        LanguageId::Spanish => IconId::LipisFlagIcons4X3Es,
                        LanguageId::French => IconId::LipisFlagIcons4X3Fr,
                        LanguageId::Italian => IconId::LipisFlagIcons4X3It,
                        LanguageId::Arabic => IconId::LipisFlagIcons4X3Ye,
                        LanguageId::Japanese => IconId::LipisFlagIcons4X3Jp,
                        LanguageId::Russian => IconId::LipisFlagIcons4X3Ru,
                        LanguageId::Vietnamese => IconId::LipisFlagIcons4X3Vn,
                        LanguageId::SimplifiedChinese => IconId::LipisFlagIcons4X3Cn,
                    }} width={String::from("2rem")} height={String::from("2rem")} title={t().settings_language_hint()}/>
            }
        </div>
    }
}
