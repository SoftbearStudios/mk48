use crate::translation::{t, Translation};
use crate::Ctw;
use gloo::timers::callback::Timeout;
use stylist::yew::styled_component;
use web_sys::{window, MouseEvent};
use yew::{html, use_state, Callback, Properties};

#[derive(PartialEq, Properties)]
pub struct InvitationLinkProps;

#[styled_component(InvitationLink)]
pub fn invitation_link(_props: &InvitationLinkProps) -> Html {
    let timeout = use_state::<Option<Timeout>, _>(|| None);

    let onclick = {
        let timeout = timeout.clone();
        let created_invitation_id = Ctw::use_core_state().created_invitation_id;

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
                let _ = clipboard.write_text(&invitation_link);

                let timeout_clone = timeout.clone();

                timeout.set(Some(Timeout::new(5000, move || {
                    timeout_clone.set(None);
                })));
            }
        })
    };

    let mut style = String::from("color: white;");

    let (contents, opacity) = if timeout.is_some() {
        (
            t().invitation_copied_label(),
            "opacity: 0.6; cursor: default;",
        )
    } else {
        (t().invitation_label(), "opacity: 1.0; cursor: pointer;")
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
