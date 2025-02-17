// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::Mk48Route;
use kodiak_client::{
    markdown, translated_text, use_game_constants, use_translator, MarkdownOptions, NexusDialog,
    RouteLink,
};
use yew::{function_component, html, Html};

#[function_component(HelpDialog)]
pub fn help_dialog() -> Html {
    let t = use_translator();
    let game_constants = use_game_constants();
    let md = translated_text!(t, "help_md");
    let components = Box::new(|href: &str, content: &str| {
        if href == "/ships/" {
            Some(html! {
                <RouteLink<Mk48Route> route={Mk48Route::Ships}>{content.to_owned()}</RouteLink<Mk48Route>>
            })
        } else {
            None
        }
    });
    html! {
        <NexusDialog title={t.help_title(game_constants)}>
            {markdown(&md, &MarkdownOptions{components, ..Default::default()})}
        </NexusDialog>
    }
}
