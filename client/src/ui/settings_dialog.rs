// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::settings::Mk48Settings;
use crate::Mk48Game;
use client_util::browser_storage::BrowserStorages;
use client_util::setting::CommonSettings;
use core_protocol::dto::ServerDto;
use core_protocol::id::ServerId;
use std::str::FromStr;
use stylist::yew::styled_component;
use web_sys::{HtmlSelectElement, InputEvent};
use yew::virtual_dom::AttrValue;
use yew::{html, html_nested, Html, TargetCast};
use yew_frontend::dialog::dialog::Dialog;
use yew_frontend::frontend::{Ctw, Gctw};
use yew_frontend::translation::{t, Translation};

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

    let t = t();
    let ctw = Ctw::use_ctw();
    let core_state = Ctw::use_core_state();
    let recreate_renderer_callback = ctw.recreate_renderer_callback;
    let gctw = Gctw::<Mk48Game>::use_gctw();

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
        let recreate_renderer_callback = recreate_renderer_callback.clone();
        gctw.change_settings_callback.reform(move |_| {
            let recreate_renderer_callback = recreate_renderer_callback.clone();
            Box::new(
                move |settings: &mut Mk48Settings, browser_storages: &mut BrowserStorages| {
                    settings.set_animations(!animations, browser_storages);
                    recreate_renderer_callback.emit(());
                },
            )
        })
    };

    let wave_quality = gctw.settings_cache.wave_quality;
    let on_set_wave_quality = {
        let recreate_renderer_callback = recreate_renderer_callback.clone();
        gctw.change_settings_callback
            .reform(move |event: InputEvent| {
                let recreate_renderer_callback = recreate_renderer_callback.clone();
                let value = event.target_unchecked_into::<HtmlSelectElement>().value();
                Box::new(
                    move |settings: &mut Mk48Settings, browser_storages: &mut BrowserStorages| {
                        if let Ok(wave_quality) = u8::from_str(&value) {
                            settings.set_wave_quality(wave_quality, browser_storages);
                            recreate_renderer_callback.emit(());
                        }
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

    let antialias = ctw.setting_cache.antialias;
    let on_toggle_antialias = {
        let recreate_renderer_callback = recreate_renderer_callback.clone();
        ctw.change_common_settings_callback.reform(move |_| {
            let recreate_renderer_callback = recreate_renderer_callback.clone();
            Box::new(
                move |settings: &mut CommonSettings, browser_storages: &mut BrowserStorages| {
                    settings.set_antialias(!antialias, browser_storages);
                    recreate_renderer_callback.emit(());
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
                    <input type="checkbox" checked={fps_shown} oninput={on_toggle_fps}/>
                    {"Show FPS Counter"}
                </label>

                <label class={label_style.clone()}>
                    <input type="checkbox" checked={chat_dialog_shown} oninput={on_toggle_chat}/>
                    {"Show Radio"}
                </label>

                <label class={label_style.clone()}>
                    <input type="checkbox" checked={cinematic} oninput={on_toggle_cinematic}/>
                    {"Cinematic Mode"}
                </label>

                <select
                    value={selected_server_id.map(|s| AttrValue::Owned(s.to_string())).unwrap_or(AttrValue::Static("unknown"))}
                    oninput={on_select_server_id}
                    class={select_style.clone()}
                >
                    if selected_server_id.is_none() || core_state.servers.is_empty() {
                        <option value="unknown">{"Unknown server"}</option>
                    }
                    {core_state.servers.values().map(|&ServerDto{server_id, region_id, player_count}| {
                        let region_str = region_id.as_human_readable_str();
                        html_nested!{
                            <option value={server_id.0.to_string()}>
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

                <select
                    value={wave_quality.to_string()}
                    oninput={on_set_wave_quality}
                    class={select_style.clone()}
                >
                    <option value={0}>{"No Waves"}</option>
                    <option value={1}>{"Good Waves"}</option>
                    <option value={2}>{"Great Waves"}</option>
                    <option value={3}>{"Fantastic Waves"}</option>
                </select>
    /*
                <select value={$resolution} on:change={e => resolution.set(parseFloat(e.target.value))}>
                    {#each [1.0, 0.5] as res}
                        <option value={res}>{res * 100}% Resolution</option>
                    {/each}
                </select>

                {#if (pendingWaveQuality !== undefined && pendingWaveQuality != $waveQuality) || (pendingAnimations !== undefined && pendingAnimations != $animations) || (pendingAntialias !== undefined && pendingAntialias != $antialias)}
                    <button on:click={applyChanges}>Apply Changes</button>
                {/if}

             */
            </Dialog>
        }
}
