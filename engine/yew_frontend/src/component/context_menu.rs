// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use crate::Ctw;
use web_sys::MouseEvent;
use yew::{function_component, html, Callback, Html};

#[derive(Clone, PartialEq)]
pub struct ContextMenuProps {
    pub html: Html,
}

#[function_component(ContextMenu)]
pub fn context_menu() -> Html {
    html! {
        if let Some(context_menu) = Ctw::use_ctw().context_menu {
            <>{context_menu.html.clone()}</>
        }
    }
}

/// Returns oncontextmenu callback that dismisses existing context menu.
pub fn dismiss_context_menu() -> Option<Callback<MouseEvent>> {
    if Ctw::use_ctw().context_menu.is_none() {
        None
    } else {
        let set_context_menu_callback = Ctw::use_set_context_menu_callback();
        Some(Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();
            set_context_menu_callback.emit(None)
        }))
    }
}
