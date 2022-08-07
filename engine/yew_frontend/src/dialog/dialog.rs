// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use crate::component::x_button::XButton;
use crate::Route;
use stylist::yew::styled_component;
use yew::prelude::*;
use yew::virtual_dom::AttrValue;
use yew_router::history::History;
use yew_router::hooks::use_history;

#[derive(PartialEq, Properties)]
pub struct DialogProps {
    pub onclick: Option<Callback<MouseEvent>>,
    pub children: Children,
    pub title: AttrValue,
}

#[styled_component(Dialog)]
pub fn dialog(props: &DialogProps) -> Html {
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
        height: 90%;
        overflow-y: auto;
        padding-left: 0.75rem;
        padding-right: 0.75rem;
        text-align: left;
        top: 10%;
        position: absolute;
        "#
    );

    let onclick = {
        let navigator = use_history().unwrap();

        Callback::from(move |_| {
            navigator.push(Route::Home);
        })
    };

    html! {
        <div id={"curtain"} onclick={onclick.clone()} class={curtain_style}>
            <div id={"modal"} onclick={Callback::from(|e: MouseEvent| e.stop_propagation())} class={modal_style}>
                <div id={"header"} class={header_style}>
                    <h2>{props.title.clone()}</h2>
                </div>
                <div id={"content"} class={content_style}>
                    {props.children.clone()}
                </div>
                <XButton {onclick}/>
            </div>
        </div>
    }
}
