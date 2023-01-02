// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::settings::{Mk48Settings, ShadowSetting};
use crate::ui::UiEvent;
use crate::Mk48Game;
use client_util::browser_storage::BrowserStorages;
use client_util::setting::CommonSettings;
use core_protocol::dto::ServerDto;
use core_protocol::id::ServerId;
use std::str::FromStr;
use stylist::yew::styled_component;
use web_sys::{HtmlSelectElement, InputEvent};
use yew::{html, html_nested, Html, TargetCast};
use yew_frontend::dialog::dialog::Dialog;
use yew_frontend::frontend::{use_core_state, use_ctw, use_gctw};
use yew_frontend::translation::{use_translation, Translation};

#[styled_component(SettingsDialog)]
pub fn settings_dialog() -> Html {
    let label_style = css! {
        r#"
        display: block;
		user-select: none;
		margin-bottom: 0.4em;
        "#
    };

    let select_style = css! {
        r#"
        border-radius: 0.25em;
        box-sizing: border-box;
        cursor: pointer;
        font-size: 1em;
        font-weight: bold;
        outline: 0;
        padding: 0.7em;
        pointer-events: all;
        white-space: nowrap;
        margin-top: 0.25em;
        border: 0;
        color: white;
	    background-color: #0075ff;
	    display: block;
        "#
    };

    let t = use_translation();
    let ctw = use_ctw();
    let core_state = use_core_state();
    let gctw = use_gctw::<Mk48Game>();
    let graphics_callback = gctw
        .send_ui_event_callback
        .reform(|_| UiEvent::GraphicsSettingsChanged);

    let cinematic = gctw.settings_cache.cinematic;
    let on_toggle_cinematic = gctw.change_settings_callback.reform(move |_| {
        Box::new(
            move |settings: &mut Mk48Settings, browser_storages: &mut BrowserStorages| {
                settings.set_cinematic(!cinematic, browser_storages);
            },
        )
    });

    let fps_shown = gctw.settings_cache.fps_shown;
    let on_toggle_fps = gctw.change_settings_callback.reform(move |_| {
        Box::new(
            move |settings: &mut Mk48Settings, browser_storages: &mut BrowserStorages| {
                settings.set_fps_shown(!fps_shown, browser_storages);
            },
        )
    });

    let animations = gctw.settings_cache.animations;
    let on_toggle_animations = {
        let graphics_callback = graphics_callback.clone();
        gctw.change_settings_callback.reform(move |_| {
            let graphics_callback = graphics_callback.clone();
            Box::new(
                move |settings: &mut Mk48Settings, browser_storages: &mut BrowserStorages| {
                    settings.set_animations(!animations, browser_storages);
                    graphics_callback.emit(());
                },
            )
        })
    };

    let dynamic_waves = gctw.settings_cache.dynamic_waves;
    let on_toggle_dynamic_waves = {
        let graphics_callback = graphics_callback.clone();
        gctw.change_settings_callback.reform(move |_| {
            let graphics_callback = graphics_callback.clone();
            Box::new(
                move |settings: &mut Mk48Settings, browser_storages: &mut BrowserStorages| {
                    settings.set_dynamic_waves(!dynamic_waves, browser_storages);
                    graphics_callback.emit(());
                },
            )
        })
    };

    let shadows = gctw.settings_cache.shadows;
    let on_set_shadows = {
        let graphics_callback = graphics_callback.clone();
        gctw.change_settings_callback
            .reform(move |event: InputEvent| {
                let graphics_callback = graphics_callback.clone();
                let value = event.target_unchecked_into::<HtmlSelectElement>().value();
                Box::new(
                    move |settings: &mut Mk48Settings, browser_storages: &mut BrowserStorages| {
                        let s = ShadowSetting::from_str(&value).unwrap();
                        settings.set_shadows(s, browser_storages);
                        graphics_callback.emit(());
                    },
                )
            })
    };

    let chat_dialog_shown = ctw.setting_cache.chat_dialog_shown;
    let on_toggle_chat = ctw.change_common_settings_callback.reform(move |_| {
        Box::new(
            move |settings: &mut CommonSettings, browser_storages: &mut BrowserStorages| {
                settings.set_chat_dialog_shown(!chat_dialog_shown, browser_storages);
            },
        )
    });

    let circle_hud = gctw.settings_cache.circle_hud;
    let on_toggle_circle_hud = gctw.change_settings_callback.reform(move |_| {
        Box::new(
            move |settings: &mut Mk48Settings, browser_storages: &mut BrowserStorages| {
                settings.set_circle_hud(!circle_hud, browser_storages);
            },
        )
    });

    let high_contrast = ctw.setting_cache.high_contrast;
    let on_toggle_high_contrast = ctw.change_common_settings_callback.reform(move |_| {
        Box::new(
            move |settings: &mut CommonSettings, browser_storages: &mut BrowserStorages| {
                settings.set_high_contrast(!high_contrast, browser_storages);
            },
        )
    });

    let antialias = ctw.setting_cache.antialias;
    let on_toggle_antialias = {
        let graphics_callback = graphics_callback.clone();
        ctw.change_common_settings_callback.reform(move |_| {
            let graphics_callback = graphics_callback.clone();
            Box::new(
                move |settings: &mut CommonSettings, browser_storages: &mut BrowserStorages| {
                    settings.set_antialias(!antialias, browser_storages);
                    graphics_callback.emit(());
                },
            )
        })
    };

    let selected_server_id = ctw.setting_cache.server_id;
    let on_select_server_id = {
        ctw.set_server_id_callback.reform(move |event: InputEvent| {
            let value = event.target_unchecked_into::<HtmlSelectElement>().value();
            ServerId::from_str(&value).ok()
        })
    };

    html! {
        <Dialog title={t.settings_title()}>
            <h3>{"General"}</h3>

            <label class={label_style.clone()}>
                <input type="checkbox" checked={cinematic} oninput={on_toggle_cinematic}/>
                {"Cinematic Mode"}
            </label>

            <label class={label_style.clone()}>
                <input type="checkbox" checked={circle_hud} disabled={cinematic} oninput={on_toggle_circle_hud}/>
                {"Circle HUD"}
            </label>

            <label class={label_style.clone()}>
                <input type="checkbox" checked={high_contrast} oninput={on_toggle_high_contrast}/>
                {"High Contrast"}
            </label>

            <label class={label_style.clone()}>
                <input type="checkbox" checked={fps_shown} oninput={on_toggle_fps}/>
                {"FPS Counter"}
            </label>

            <label class={label_style.clone()}>
                <input type="checkbox" checked={chat_dialog_shown} oninput={on_toggle_chat}/>
                {"Radio"}
            </label>

            <select
                oninput={on_select_server_id}
                class={select_style.clone()}
            >
                if selected_server_id.is_none() || core_state.servers.is_empty() {
                    <option value="unknown" selected={true}>{"Unknown server"}</option>
                }
                {core_state.servers.values().map(|&ServerDto{server_id, region_id, player_count}| {
                    let region_str = region_id.as_human_readable_str();
                    html_nested!{
                        <option value={server_id.0.to_string()} selected={selected_server_id == Some(server_id)}>
                            {format!("Server {server_id} - {region_str} ({player_count} players)")}
                        </option>
                    }
                }).collect::<Html>()}
            </select>

            <h3>{"Graphics"}</h3>

            <label class={label_style.clone()}>
                <input type="checkbox" checked={animations} oninput={on_toggle_animations}/>
                {"Animations"}
            </label>

            <label class={label_style.clone()}>
                <input type="checkbox" checked={antialias} oninput={on_toggle_antialias}/>
                {"Antialiasing"}
            </label>

            <label class={label_style.clone()}>
                <input type="checkbox" checked={dynamic_waves} oninput={on_toggle_dynamic_waves}/>
                {"Dynamic Waves"}
            </label>

            <select
                oninput={on_set_shadows}
                class={select_style.clone()}
            >
                {[(ShadowSetting::None, "No Shadows"), (ShadowSetting::Hard, "Hard Shadows"), (ShadowSetting::Soft, "Soft Shadows")].into_iter().map(|(v, d)| html_nested!{
                    <option value={v.to_string()} selected={shadows == v}>{d}</option>
                }).collect::<Html>()}
            </select>
        </Dialog>
    }
}
