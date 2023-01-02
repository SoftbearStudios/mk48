// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::armament::{group_armaments, Group};
use crate::translation::Mk48Translation;
use crate::ui::sprite::Sprite;
use common::altitude::Altitude;
use common::entity::{EntityData, EntityKind, EntityType};
use common::ticks::Ticks;
use common::velocity::Velocity;
use core_protocol::id::LanguageId;
use stylist::yew::styled_component;
use stylist::StyleSource;
use yew::{html, html_nested, Html};
use yew_frontend::component::link::Link;
use yew_frontend::dialog::dialog::Dialog;
use yew_frontend::translation::use_translation;

#[styled_component(ShipsDialog)]
pub fn ships_dialog() -> Html {
    let t = use_translation();
    let table_style = css!(
        r#"
        border-spacing: 1em;
		text-align: left;
		width: 100%;
		"#
    );

    html! {
        <Dialog title={"Ships"}>
            <p>{"The following is a list of all ships in the game, and their weapons. Note that certain values are approximate and may be affected by other factors. For example, weapon damage depends on hit location."}</p>

            <table>
                {EntityType::iter().filter(|t| t.data().kind == EntityKind::Boat).map(|entity_type| html_nested!{
                    <tr>
                        <td>
                            {entity_card(t, &table_style, entity_type, None)}
                        </td>
                    </tr>
                }).collect::<Html>()}
            </table>
        </Dialog>
    }
}

fn entity_card(
    t: LanguageId,
    table_style: &StyleSource,
    entity_type: EntityType,
    count: Option<u8>,
) -> Html {
    let data: &'static EntityData = entity_type.data();
    html! {
        <table class={table_style.clone()}>
            <tr>
                <td>
                    <h3>
                        {data.label.clone()}
                        if let Some(count) = count {
                            {format!(" × {count}")}
                        }
                    </h3>
                    <i>
                        if data.kind == EntityKind::Boat {
                            {format!("Level {} ", data.level)}
                        }
                        {t.entity_kind_name(data.kind, data.sub_kind)}
                    </i>
                    if let Some(href) = data.link.clone() {
                        {" ("} <Link {href}>{"Learn more"}</Link>{")"}
                    }
                </td>
                <td rowspan="2">
                    <ul>
                        if data.length != 0.0 {
                            <li>{format!("Length: {:.1}m", data.length)}</li>
                        }
                        if data.draft != Altitude::ZERO {
                            <li>{format!("Draft: {:.1}m", data.draft.to_meters())}</li>
                        }
                        if data.speed != Velocity::ZERO {
                            <li>{format!("Speed: {:.1}m/s ({:.1}kn)", data.speed.to_mps(), data.speed.to_knots())}</li>
                        }
                        if data.range != 0.0 {
                            <li>{format!("Range: {}m", data.range as u32)}</li>
                        }
                        if data.depth != Altitude::ZERO {
                            <li>{format!("Max Depth: {}m", data.depth.to_meters() as u16)}</li>
                        }
                        if data.lifespan != Ticks::ZERO {
                            <li>{format!("Lifespan: {:.1}s", data.lifespan.to_secs())}</li>
                        }
                        if data.reload != Ticks::ZERO {
                            <li>{format!("Reload: {:.1}s", data.reload.to_secs())}</li>
                        }
                        if data.damage != 0.0 {
                            <li>{format!("{}: {:.2}", if data.kind == EntityKind::Boat { "Health" } else { "Damage" }, data.damage)}</li>
                        }
                        if data.anti_aircraft != 0.0 {
                            <li>{format!("Anti-Aircraft: {:.2}", data.anti_aircraft)}</li>
                        }
                        if data.torpedo_resistance != 0.0 {
                            <li>{format!("Torpedo Resistance: {}%", (data.torpedo_resistance * 100.0) as u16)}</li>
                        }
                        if data.stealth != 0.0 {
                            <li>{format!("Stealth: {}%", (data.stealth * 100.0) as u16)}</li>
                        }
                        if data.npc {
                            <li>{"NPC only"}</li>
                        }
                    </ul>
                </td>
            </tr>
            <tr>
                <td>
                    <Sprite {entity_type}/>
                </td>
            </tr>
            {group_armaments(&data.armaments, &[]).into_iter().map(|Group{entity_type, total, ..}| html_nested!{
                <tr>
                    <td colspan="2">
                        {entity_card(t, table_style, entity_type, Some(total))}
                    </td>
                </tr>
            }).collect::<Html>()}
        </table>
    }
}
