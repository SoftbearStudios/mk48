// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use stylist::yew::styled_component;
use yew::{html, Html};

#[styled_component(Spinner)]
pub fn spinner() -> Html {
    let progress_spinner_css_class = css!(
        r#"
        display: inline-block;
        "#
    );

    let animation_css_class = css!(
        r#"
		animation: spin 2s linear infinite;
		border: 5px solid #f3f3f3;
		border-radius: 50%;
		border-top: 5px solid #3b99fc;
		box-sizing: border-box;
		height: 10vh;
		width: 10vh;

		@keyframes spin {
            0% {
                transform: rotate(0deg);
            }
            100% {
                transform: rotate(360deg);
            }
        }
        "#
    );

    html! {
        <div class={progress_spinner_css_class}>
            <div class={animation_css_class}/>
        </div>
    }
}
