// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::frontend::{use_change_common_settings_callback, use_ctw};
use crate::translation::{use_translation, Translation};
use web_sys::MouseEvent;
use yew::virtual_dom::AttrValue;
use yew::{function_component, html, Callback, Html, Properties};
use yew_icons::{Icon, IconId};

#[derive(PartialEq, Properties)]
pub struct VolumeIconProps {
    #[prop_or("2rem".into())]
    pub size: AttrValue,
}

#[function_component(VolumeIcon)]
pub fn volume_icon(props: &VolumeIconProps) -> Html {
    let volume = use_ctw().setting_cache.volume;
    let current = ((volume * 2.0).round() as u8).clamp(0, 2);

    let onclick = {
        let change_common_settings_callback = use_change_common_settings_callback();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            change_common_settings_callback.emit(Box::new(
                move |common_settings, browser_storages| {
                    let next = (current + 1) % 3;
                    common_settings.set_volume(next as f32 / 2.0, browser_storages);
                },
            ));
        })
    };

    let oncontextmenu = {
        let change_common_settings_callback = use_change_common_settings_callback();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            change_common_settings_callback.emit(Box::new(
                move |common_settings, browser_storages| {
                    let next = (current + 2) % 3;
                    common_settings.set_volume(next as f32 / 2.0, browser_storages);
                },
            ));
        })
    };

    let (icon_id, style) = match current {
        0 => (IconId::BootstrapVolumeMute, "opacity: 0.6;"),
        1 => (IconId::BootstrapVolumeDownFill, "opacity: 1;"),
        2 => (IconId::BootstrapVolumeUpFill, "opacity: 1;"),
        _ => unreachable!(),
    };
    let t = use_translation();

    html! {
        <Icon
            {icon_id}
            title={t.settings_volume_hint()}
            {onclick}
            {oncontextmenu}
            width={props.size.clone()}
            height={props.size.clone()}
            style={format!("color: white; cursor: pointer; user-select: none; vertical-align: bottom; {}", style)}
        />
    }
}
