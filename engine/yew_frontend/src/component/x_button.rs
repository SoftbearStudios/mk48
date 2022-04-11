// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use web_sys::MouseEvent;
use yew::prelude::*;

#[derive(Default, PartialEq, Properties)]
pub struct XButtonProps {
    pub onclick: Callback<MouseEvent>,
}

#[function_component(XButton)]
pub fn x_button(props: &XButtonProps) -> Html {
    let style = concat!(
        "background-color: #bf0f0f;",
        "border: 1px solid #bf0f0f;",
        "font-size: 1rem;",
        "font-weight: bold;",
        "position: absolute;",
        "right: 0.5em;",
        "top: 0;",
        "width: 2.2em;",
    );
    html! {
        <button onclick={props.onclick.clone()} style={style}>{"X"}</button>
    }
}
