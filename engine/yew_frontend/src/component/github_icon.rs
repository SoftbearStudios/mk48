// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::link_icon::LinkIcon;
use yew::virtual_dom::AttrValue;
use yew::{function_component, html, Html, Properties};
use yew_icons::IconId;

#[derive(PartialEq, Properties)]
pub struct GithubIconProps {
    /// Github repository link.
    pub repository_link: AttrValue,
    #[prop_or("2.5rem".into())]
    pub size: AttrValue,
}

#[function_component(GithubIcon)]
pub fn github_icon(props: &GithubIconProps) -> Html {
    html! {
        <LinkIcon icon_id={IconId::BootstrapGithub} title={"GitHub"} link={props.repository_link.clone()} size={props.size.clone()}/>
    }
}
