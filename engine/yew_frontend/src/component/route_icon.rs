// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use web_sys::MouseEvent;
use yew::virtual_dom::AttrValue;
use yew::{function_component, html, Callback, Html, Properties};
use yew_icons::{Icon, IconId};
use yew_router::hooks::use_navigator;
use yew_router::Routable;

#[derive(PartialEq, Properties)]
pub struct RouteIconProps<R: Routable> {
    pub icon_id: IconId,
    pub title: Option<AttrValue>,
    pub route: R,
    #[prop_or("2.5rem".into())]
    pub size: AttrValue,
}

#[function_component(RouteIcon)]
pub fn route_icon<R: Routable + Copy + 'static>(props: &RouteIconProps<R>) -> Html {
    let onclick = {
        let route = props.route;
        let navigator = use_navigator().unwrap();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            navigator.push(&route);
        })
    };

    html! {
        <Icon icon_id={props.icon_id} title={props.title.clone()} {onclick} width={props.size.clone()} height={props.size.clone()} style={"color: white; cursor: pointer; user-select: none; vertical-align: bottom;"}/>
    }
}
