// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use crate::component::x_button::XButton;
use crate::window::location::set_location_href;
use yew::prelude::*;

#[derive(Default, PartialEq, Properties)]
pub struct DialogProps {
    pub onclick: Option<Callback<MouseEvent>>,
    pub children: Children,
    pub title: String,
}

#[function_component(Dialog)]
pub fn dialog(props: &DialogProps) -> Html {
    let content_style = concat!(
        "height: 90%;",
        "overflow-y: auto;",
        "padding-right: 1rem;",
        "text-align: left;",
    );

    let curtain_style = concat!(
        "background-color: #0003;",
        "bottom: 0;",
        "left: 0;",
        "position: absolute;",
        "right: 0;",
        "top: 0;",
    );

    let header_style = concat!("height: 10%;", "left: 0;", "top: 0;", "right: 0;",);

    let modal_style = concat!(
        "background-color: #174479;",
        "border-radius: 0.5em;",
        "bottom: 0px;",
        "box-shadow: 5px 5px 5px #00000020;",
        "color: white;",
        "inset: 10%;",
        "padding: 15px;",
        "position: absolute;",
        "text-align: center;",
    );

    html! {
        <div id={"curtain"} onclick={|_| set_location_href("#/")} style={curtain_style}>
            <div id={"modal"} onclick={Callback::from(|e: MouseEvent| e.stop_propagation())} style={modal_style}>
                <div id={"content"} style={header_style}>
                    <h2>{props.title.clone()}</h2>
                </div>
                <div id={"content"} style={content_style}>
                    {props.children.clone()}
                </div>
                <XButton onclick={Callback::from(|_| set_location_href("#/"))}/>
            </div>
        </div>
    }
}
