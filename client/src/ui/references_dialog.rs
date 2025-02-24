// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use kodiak_client::{
    markdown, translate, use_features, use_translator, Link, MarkdownOptions, NexusDialog,
};
use yew::{function_component, html, Html};

#[function_component(ReferencesDialog)]
pub fn references_dialog() -> Html {
    let credits = use_features().outbound.credits;
    let t = use_translator();

    let components = Box::new(move |href: &str, content: &str| {
        Some(html! {
            <Link href={href.to_owned()} enabled={credits}>{content.to_owned()}</Link>
        })
    });

    html! {
        <NexusDialog title={translate!(t, "References")}>
            {markdown(include_str!("../../../assets/models/README.md"), &MarkdownOptions{components, ..Default::default()})}
        </NexusDialog>
    }
}
