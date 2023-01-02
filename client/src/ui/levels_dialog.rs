// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::sprite::Sprite;
use common::entity::{EntityData, EntityKind, EntityType};
use common::util::level_to_score;
use stylist::yew::styled_component;
use yew::{html, html_nested, Html};
use yew_frontend::component::positioner::Align;
use yew_frontend::dialog::dialog::Dialog;
use yew_frontend::translation::{use_translation, Translation};

#[styled_component(LevelsDialog)]
pub fn levels_dialog() -> Html {
    let sprite_style = css!(
        r#"
        display: inline-block;
		margin: 0.5em;
        "#
    );

    let t = use_translation();

    html! {
        <Dialog title={"Levels"} align={Align::Center}>
            {(1..=EntityData::MAX_BOAT_LEVEL).map(move |level| html_nested!{
                <div>
                    <h3>{format!("Level {} ({})", level, t.score(level_to_score(level)))}</h3>
                    {EntityType::iter().filter(move |entity_type| entity_type.data().kind == EntityKind::Boat && entity_type.data().level == level).map(|entity_type| {
                        html_nested! {
                            <Sprite {entity_type} class={sprite_style.clone()}/>
                        }
                    }).collect::<Html>()}
                </div>
            }).collect::<Html>()}
        </Dialog>
    }
}
