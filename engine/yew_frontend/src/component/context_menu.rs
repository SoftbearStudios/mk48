// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::frontend::use_set_context_menu_callback;
use crate::WindowEventListener;
use gloo::timers::callback::Timeout;
use stylist::yew::styled_component;
use web_sys::MouseEvent;
use yew::{
    function_component, hook, html, use_effect_with_deps, Callback, Children, Html, Properties,
};

#[derive(Clone, PartialEq, Properties)]
pub struct ContextMenuProps {
    pub event: MouseEvent,
    pub children: Children,
}

#[function_component(ContextMenu)]
pub fn context_menu(props: &ContextMenuProps) -> Html {
    let style = format!("background-color: #444444aa; min-width: 100px; position: absolute; display: flex; flex-direction: column; left: {}px; top: {}px;", props.event.x(), props.event.y());

    // Provide for closing the menu by rightclicking elsewhere.
    let set_context_menu_callback = use_set_context_menu_callback();
    let set_context_menu_callback_clone = set_context_menu_callback.clone();
    use_effect_with_deps(
        |_| {
            let listener = WindowEventListener::new_body(
                "contextmenu",
                move |e: &MouseEvent| {
                    e.prevent_default();
                    e.stop_propagation();
                    set_context_menu_callback.emit(None)
                },
                true,
            );
            let timeout = Timeout::new(5000, move || {
                set_context_menu_callback_clone.emit(None);
            });
            || drop((listener, timeout))
        },
        props.event.clone(),
    );

    html! {
        <div {style}>
            {props.children.clone()}
        </div>
    }
}

#[derive(Clone, PartialEq, Properties)]
pub struct ContextMenuButtonProps {
    pub children: Children,
    pub onclick: Option<Callback<MouseEvent>>,
}

#[styled_component(ContextMenuButton)]
pub fn context_menu_button(props: &ContextMenuButtonProps) -> Html {
    let class = css!(
        r#"
		color: white;
		background-color: #444444aa;
		border: 0;
		border-radius: 0;
		outline: 0;
		margin: 0;
		padding: 5px;

        :hover {
            filter: brightness(1.1);
        }

        :hover:active {
            filter: brightness(1.05);
        }
    "#
    );

    let set_context_menu_callback = use_set_context_menu_callback();
    let onclick = props.onclick.clone().map(move |onclick| {
        onclick.reform(move |e| {
            // Close the menu when an option is clicked.
            set_context_menu_callback.emit(None);
            e
        })
    });

    html! {
        <button {onclick} {class}>
            {props.children.clone()}
        </button>
    }
}

/// Returns oncontextmenu callback that dismisses existing context menu.
#[hook]
pub fn use_dismiss_context_menu() -> Callback<MouseEvent> {
    let set_context_menu_callback = use_set_context_menu_callback();
    Callback::from(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();
        set_context_menu_callback.emit(None)
    })
}
