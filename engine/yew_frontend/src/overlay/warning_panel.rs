// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use stylist::yew::styled_component;
use yew::{html, Properties};

#[derive(Default, PartialEq, Properties)]
pub struct WarningProps {
    pub message: Option<String>,
}

#[styled_component(WarningOverlay)]
pub fn chat_overlay(props: &WarningProps) -> Html {

    let onclick = || {
/*
        try {
            await fetch("/");
            location.reload();
        } catch (err) {
            // No-op.
        }
*/
    };

    let div_css_class = css!{
        r#"
        background-color: #f6f6f6;
        border-radius: 1rem;
        box-shadow: 0em 0.25rem 0 #cccccc;
        color: #000000;
        font-size: 2rem;
        left: 50%;
        max-width: 60%;
        padding: 2rem;
        position: absolute;
        text-align: center;
        top: 50%;
        transform: translate(-50%, -50%);
        word-break: break-word;
        "#
    };

    html!{
        <div id="warning_panel" class={div_css_class}>
            if let Some(message) = &props.message {
                {message}
            }
            <button onclick={|_| onclick()}>{"Refresh"}</button>
        </div>
    }
}
