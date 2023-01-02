// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use stylist::yew::styled_component;
use web_sys::MouseEvent;
use yew::{classes, html, Callback, Children, Classes, Html, Properties};
use yew_router::hooks::use_navigator;
use yew_router::Routable;

#[derive(PartialEq, Properties)]
pub struct RouteLinkProps<R: Routable> {
    pub children: Children,
    pub route: R,
    #[prop_or_default]
    pub class: Classes,
}

#[styled_component(RouteLink)]
pub fn route_link<R: Routable + Clone + 'static>(props: &RouteLinkProps<R>) -> Html {
    let style = css!(
        r#"
        color: white;
        cursor: pointer;
        user-select: none;
        user-drag: none;
        -webkit-user-drag: none;
        "#
    );

    let onclick = {
        let route = props.route.clone();
        let navigator = use_navigator().unwrap();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();
            navigator.push(&route);
        })
    };

    // Trick yew into not warning about bad practice.
    let href: &'static str = "javascript:void(0)";

    html! {
        <a {href} {onclick} class={classes!(style, props.class.clone())}>
            {props.children.clone()}
        </a>
    }
}
