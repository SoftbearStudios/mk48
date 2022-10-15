// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use stylist::yew::styled_component;
use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct CurtainProps {
    pub children: Children,
    pub onclick: Option<Callback<MouseEvent>>,
}

#[styled_component(Curtain)]
pub fn curtain(props: &CurtainProps) -> Html {
    let curtain_style = css!(
        r#"
        background-color: #0003;
        bottom: 0;
        left: 0;
        position: absolute;
        right: 0;
        top: 0;
    "#
    );

    html! {
        <div onclick={props.onclick.clone()} class={curtain_style}>
            {props.children.clone()}
        </div>
    }
}
