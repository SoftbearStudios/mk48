// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::curtain::Curtain;
use crate::component::positioner::Align;
use crate::component::route_link::RouteLink;
use crate::component::x_button::XButton;
use crate::frontend::use_ctw;
use stylist::yew::styled_component;
use web_sys::window;
use yew::prelude::*;
use yew::virtual_dom::AttrValue;
use yew_router::hooks::use_navigator;
use yew_router::AnyRoute;

#[derive(PartialEq, Properties)]
pub struct DialogProps {
    pub onclick: Option<Callback<MouseEvent>>,
    pub children: Children,
    pub title: AttrValue,
    #[prop_or(Align::Left)]
    pub align: Align,
}

#[styled_component(Dialog)]
pub fn dialog(props: &DialogProps) -> Html {
    let modal_style = css!(
        r#"
        background-color: #174479;
        border-radius: 0.5em;
        box-shadow: 5px 5px 5px #00000020;
        color: white;
        top: 10%;
        left: 10%;
        right: 10%;
        bottom: 10%;
        position: absolute;
        text-align: center;
        user-select: text;
    "#
    );

    let header_style = css!(
        r#"
        height: 10%;
        left: 0;
        top: 0;
        right: 0;
        position: absolute;
    "#
    );

    let content_style = css!(
        r#"
        height: 85%;
        overflow-y: auto;
        padding-left: 0.75rem;
        padding-right: 0.75rem;
        width: calc(100% - 1.5em);
        top: 10%;
        position: absolute;
        "#
    );

    let footer_style = css!(
        r#"
        position: absolute;
        bottom: 0;
        left: 0;
        right: 0;
        top: 95%;
        display: flex;
        justify-content: space-evenly;
        "#
    );

    let link_style = css!(
        r#"
        background-color: #174479;
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: 100%;
        text-decoration: none;
        filter: brightness(0.8);

        :hover {
            filter: brightness(0.9);
        }
        "#
    );

    let link_selected_style = css!(
        r#"
        background-color: #174479;
        filter: none;
        cursor: default;

        :hover {
            filter: none;
        }
        "#
    );

    let onclick = {
        let navigator = use_navigator().unwrap();

        Callback::from(move |_| {
            navigator.push(&AnyRoute::new("/"));
        })
    };

    let routes = use_ctw().routes;
    let pathname = window()
        .unwrap()
        .location()
        .pathname()
        .unwrap_or_else(|_| String::from("/"));

    fn route_to_title(route: &str) -> String {
        if route.len() < 3 {
            return "".to_owned();
        }
        let name = &route[1..route.len() - 1];
        let mut title = name.to_owned();
        title[0..1].make_ascii_uppercase();
        title
    }

    html! {
        <Curtain onclick={onclick.clone()}>
            <div onclick={Callback::from(|e: MouseEvent| e.stop_propagation())} class={modal_style}>
                <div class={header_style}>
                    <h2>{props.title.clone()}</h2>
                </div>
                <div class={content_style} style={props.align.as_css()}>
                    {props.children.clone()}
                </div>
                <div class={footer_style}>
                    {routes.into_iter().map(|route| html_nested!{
                        <RouteLink<AnyRoute> route={AnyRoute::new(route)} class={classes!(link_style.clone(), (pathname.starts_with(route)).then(|| link_selected_style.clone()))}>{route_to_title(route)}</RouteLink<AnyRoute>>
                    }).collect::<Html>()}
                </div>
                <div style="position: absolute; top: 0.5rem; right: 0.5em;">
                    <XButton {onclick}/>
                </div>
            </div>
        </Curtain>
    }
}
