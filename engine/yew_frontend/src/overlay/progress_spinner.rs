// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use stylist::yew::styled_component;
use yew::html;

#[styled_component(ProgressSpinnerOverlay)]
pub fn progress_spinner() -> Html {
    let progress_spinner_css_class = css!(
        r#"
		left: 50%;
		position: absolute;
		top: 50%;
		transform: translate(-50%, -50%);
        "#
    );

    let animation_css_class = css!(
        r#"
		animation: spin 2s linear infinite;
		border: 5px solid #f3f3f3;
		border-radius: 50%;
		border-top: 5px solid #3b99fc;
		box-sizing: border-box;
		height: 20vh;
		width: 20vh;
        "#
    );

    /* TODO: in:fade
    @keyframes spin {
        0% {
            transform: rotate(0deg);
        }
        100% {
            transform: rotate(360deg);
        }
    }
    */

    html! {
        <div id="progress_spinner" class={progress_spinner_css_class}>
            <div id="animation" class={animation_css_class}/>
        </div>
    }
}
