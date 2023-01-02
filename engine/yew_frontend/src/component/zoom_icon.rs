// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::frontend::use_raw_zoom_callback;
use crate::translation::{use_translation, Translation};
use web_sys::MouseEvent;
use yew::virtual_dom::AttrValue;
use yew::{function_component, html, Callback, Html, Properties};
use yew_icons::{Icon, IconId};

#[derive(PartialEq, Properties)]
pub struct ZoomIconProps {
    pub amount: i8,
    #[prop_or("2rem".into())]
    pub size: AttrValue,
}

#[function_component(ZoomIcon)]
pub fn zoom_icon(props: &ZoomIconProps) -> Html {
    let onclick = {
        let raw_zoom_callback = use_raw_zoom_callback();
        let amount = props.amount as f32;

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            raw_zoom_callback.emit(amount);
        })
    };

    let t = use_translation();
    let (icon_id, title) = if props.amount < 0 {
        (IconId::BootstrapZoomIn, t.zoom_in_hint())
    } else {
        (IconId::BootstrapZoomOut, t.zoom_out_hint())
    };

    html! {
        <Icon {icon_id} {title} {onclick} width={props.size.clone()} height={props.size.clone()} style={"color: white; cursor: pointer; user-select: none; vertical-align: bottom;"}/>
    }
}
