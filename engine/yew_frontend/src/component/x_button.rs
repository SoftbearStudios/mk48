// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use stylist::yew::styled_component;
use web_sys::MouseEvent;
use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct XButtonProps {
    pub onclick: Callback<MouseEvent>,
}

#[styled_component(XButton)]
pub fn x_button(props: &XButtonProps) -> Html {
    let class = css!(
        r#"
        font-size: 0.8rem;
        width: 2.2em;
        font-weight: bold;
        background-color: #bf0f0f;
        border: 1px solid #bf0f0f;
        border-radius: 0.25em;
        box-sizing: border-box;
        color: white;
        cursor: pointer;
        margin: 0;
        padding: 0.5em 0.6em;
        text-decoration: none;
        white-space: nowrap;
    "#
    );
    html! {
        <button onclick={props.onclick.clone()} {class}>{"X"}</button>
    }
}
