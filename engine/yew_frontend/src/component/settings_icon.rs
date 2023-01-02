// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::route_icon::RouteIcon;
use crate::translation::{use_translation, Translation};
use yew::virtual_dom::AttrValue;
use yew::{function_component, html, Html, Properties};
use yew_icons::IconId;
use yew_router::Routable;

#[derive(PartialEq, Properties)]
pub struct SettingsIconProps<R: PartialEq> {
    pub route: R,
    #[prop_or("2rem".into())]
    pub size: AttrValue,
}

#[function_component(SettingsIcon)]
pub fn settings_icon<R: Routable + Copy + 'static>(props: &SettingsIconProps<R>) -> Html {
    let t = use_translation();
    html! {
        <RouteIcon<R> icon_id={IconId::BootstrapGear} title={t.settings_hint()} route={props.route} size={props.size.clone()}/>
    }
}
