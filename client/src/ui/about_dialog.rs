// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::Mk48Route;
use common::entity::{EntityData, EntityKind, EntityType};
use kodiak_client::{
    markdown, translated_text, use_features, use_game_constants, use_translator, EngineNexus, Link,
    MarkdownOptions, NexusDialog, RouteLink,
};
use std::collections::HashSet;
use yew::{function_component, html, Html};

#[function_component(AboutDialog)]
pub fn about_dialog() -> Html {
    let t = use_translator();
    let game_constants = use_game_constants();
    let features = use_features();
    let credits = features.outbound.credits;

    let md = translated_text!(t, "about_md");
    let components = Box::new(move |href: &str, content: &str| match href {
        "/ships/" => Some({
            let boat_type_count = EntityType::iter()
                .filter(|t| t.data().kind == EntityKind::Boat)
                .count();
            html! {
                <RouteLink<Mk48Route> route={Mk48Route::Ships}>
                    {content.replace('#', &boat_type_count.to_string())}
                </RouteLink<Mk48Route>>
            }
        }),
        "weapons" => Some({
            let weapon_sub_kind_count = EntityType::iter()
                .filter_map(|t| (t.data().kind == EntityKind::Weapon).then(|| t.data().sub_kind))
                .collect::<HashSet<_>>()
                .len();
            html! {{weapon_sub_kind_count.to_string()}}
        }),
        "/levels/" => Some(html! {
            <RouteLink<Mk48Route> route={Mk48Route::Ships}>
                {content.replace('#', &EntityData::MAX_BOAT_LEVEL.to_string())}
            </RouteLink<Mk48Route>>
        }),
        "/licensing/" => Some(html! {
            <RouteLink<EngineNexus> route={EngineNexus::Licensing}>
                {content.to_owned()}
            </RouteLink<EngineNexus>>
        }),
        _ if href.starts_with("http") => Some(html! {
            <Link href={href.to_owned()} enabled={credits}>{content.to_owned()}</Link>
        }),
        _ => None,
    });

    let markdown_options = MarkdownOptions {
        components,
        ..Default::default()
    };

    html! {
        <NexusDialog title={t.about_title(game_constants)}>
            {markdown(&md, &markdown_options)}
            {markdown(include_str!("./translations/credits.md"), &markdown_options)}
            if features.outbound.contact_info {
                {t.about_contact()}
            }
        </NexusDialog>
    }
}
