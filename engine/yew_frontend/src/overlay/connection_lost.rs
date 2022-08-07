// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use crate::component::positioner::{Position, Positioner};
use stylist::yew::styled_component;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use web_sys::{window, Request, RequestInit, RequestMode, Response};
use yew::{classes, html};

#[styled_component(ConnectionLost)]
pub fn connection_lost() -> Html {
    let connection_lost_css = css!(
        r#"
        background-color: #f6f6f6;
		border-radius: 1rem;
		box-shadow: 0em 0.25rem 0 #cccccc;
		color: #000000;
		font-size: 2rem;
		word-break: break-word;
        "#
    );

    let p_css = css!(
        r#"
        margin: 1rem;
        "#
    );

    let button_css = css! {
        r#"
        background-color: #549f57;
        border-radius: 1rem;
        border: 1px solid #61b365;
        box-sizing: border-box;
        color: white;
        cursor: pointer;
        font-size: 2rem;
        margin: 1rem;
        min-width: 12rem;
        padding-bottom: 0.7rem;
        padding-top: 0.5rem;
        text-decoration: none;
        white-space: nowrap;
        width: min-content;

        :disabled {
            filter: opacity(0.6);
        }

        :hover:not(:disabled) {
            filter: brightness(0.95);
        }

        :active:not(:disabled) {
            filter: brightness(0.9);
        }
        "#
    };

    // Refresh the page, which serves two purposes:
    // - The server may have restarted, so might need to download new client
    // - The refreshed client will attempt to regain connection
    let refresh = |_| {
        let _ = future_to_promise(async {
            // Do a pre-flight request to make sure we aren't refreshing ourselves into a browser error.
            let mut opts = RequestInit::new();
            opts.method("GET");
            opts.mode(RequestMode::Cors);

            let request = match Request::new_with_str_and_init("/", &opts) {
                Ok(request) => request,
                Err(_) => return Err(JsValue::NULL),
            };
            let window = window().unwrap();
            let response_value = match JsFuture::from(window.fetch_with_request(&request)).await {
                Ok(response_value) => response_value,
                Err(_) => return Err(JsValue::NULL),
            };
            let response: Response = match response_value.dyn_into() {
                Ok(response) => response,
                Err(_) => return Err(JsValue::NULL),
            };
            if response.ok() {
                let _ = window.location().reload();
            }
            Ok(JsValue::NULL)
        });
    };

    html! {
        <Positioner id="connection_lost" position={Position::Center} class={classes!(connection_lost_css)}>
            <p class={p_css}>{"The battle is over. Try starting again shortly."}</p>
            <button onclick={refresh} class={button_css}>{"Refresh"}</button>
        </Positioner>
    }
}
