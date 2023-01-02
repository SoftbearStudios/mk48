// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::frontend::use_core_state;
use crate::translation::{use_translation, Translation};
use gloo::timers::callback::Timeout;
use stylist::yew::styled_component;
use web_sys::{window, MouseEvent};
use yew::{hook, html, use_state, Callback, Html, Properties};

#[derive(PartialEq, Properties)]
pub struct InvitationLinkProps;

#[styled_component(InvitationLink)]
pub fn invitation_link(_props: &InvitationLinkProps) -> Html {
    let t = use_translation();
    let onclick = use_copy_invitation_link();

    let mut style = String::from("color: white;");

    let (contents, opacity) = if onclick.is_some() {
        (t.invitation_label(), "opacity: 1.0; cursor: pointer;")
    } else {
        (
            t.invitation_copied_label(),
            "opacity: 0.6; cursor: default;",
        )
    };

    style += opacity;

    // Trick yew into not warning about bad practice.
    let href: &'static str = "javascript:void(0)";

    html! {
        <a {href} {onclick} {style}>
            {contents}
        </a>
    }
}

/// [`None`] indicates the button was pressed recently.
#[hook]
pub fn use_copy_invitation_link() -> Option<Callback<MouseEvent>> {
    let timeout = use_state::<Option<Timeout>, _>(|| None);
    let created_invitation_id = use_core_state().created_invitation_id;

    timeout.is_none().then(|| {
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            let window = window().unwrap();

            if let Some((invitation_id, (origin, clipboard))) = created_invitation_id.zip(
                window
                    .location()
                    .origin()
                    .ok()
                    .zip(window.navigator().clipboard()),
            ) {
                let invitation_link = format!("{}/invite/{}", origin, invitation_id.0);

                // TODO: await this.
                let _ = clipboard.write_text(&invitation_link);

                let timeout_clone = timeout.clone();

                timeout.set(Some(Timeout::new(5000, move || {
                    timeout_clone.set(None);
                })));
            }
        })
    })
}
