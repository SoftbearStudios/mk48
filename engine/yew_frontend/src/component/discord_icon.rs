// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::link_icon::LinkIcon;
use yew::virtual_dom::AttrValue;
use yew::{function_component, html, Html, Properties};
use yew_icons::IconId;

#[derive(PartialEq, Properties)]
pub struct DiscordIconProps {
    /// Discord invite link (defaults to Softbear discord server).
    #[prop_or("https://discord.gg/YMheuFQWTX".into())]
    pub invite_link: AttrValue,
    #[prop_or("2.5rem".into())]
    pub size: AttrValue,
}

#[function_component(DiscordIcon)]
pub fn discord_icon(props: &DiscordIconProps) -> Html {
    html! {
        <LinkIcon icon_id={IconId::BootstrapDiscord} title={"Discord"} link={props.invite_link.clone()} size={props.size.clone()}/>
    }
}
