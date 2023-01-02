// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::invitation_link::use_copy_invitation_link;
use crate::translation::{use_translation, Translation};
use yew::virtual_dom::AttrValue;
use yew::{function_component, html, Html, Properties};
use yew_icons::{Icon, IconId};

#[derive(PartialEq, Properties)]
pub struct InvitationIconProps {
    #[prop_or("2rem".into())]
    pub size: AttrValue,
}

#[function_component(InvitationIcon)]
pub fn invitation_icon(props: &InvitationIconProps) -> Html {
    let t = use_translation();
    let onclick = use_copy_invitation_link();
    let (title, style) = if onclick.is_some() {
        (t.invitation_label(), "opacity: 1.0; cursor: pointer;")
    } else {
        (
            t.invitation_copied_label(),
            "opacity: 0.6; cursor: default;",
        )
    };
    html! {
        <Icon icon_id={IconId::BootstrapPersonPlus} {title} {onclick} width={props.size.clone()} height={props.size.clone()} style={format!("color: white; cursor: pointer; user-select: none; vertical-align: bottom; {}", style)}/>
    }
}
