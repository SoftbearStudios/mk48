// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::armament::{group_armaments, Group};
use crate::ui::sprite::Sprite;
use crate::ui::{Mk48Phrases, Mk48Route};
use common::altitude::Altitude;
use common::entity::{EntityData, EntityKind, EntityType};
use common::ticks::Ticks;
use common::util::level_to_score;
use common::velocity::Velocity;
use kodiak_client::yew_router::hooks::use_navigator;
use kodiak_client::{translate, use_translator, Link, NexusDialog, PathParam, Translator};
use stylist::yew::styled_component;
use stylist::StyleSource;
use web_sys::MouseEvent;
use yew::{html, html_nested, Callback, Html, Properties};

#[derive(PartialEq, Properties)]
pub struct ShipsDialogProps {
    #[prop_or(None)]
    pub selected: Option<EntityType>,
}

#[styled_component(ShipsDialog)]
pub fn ships_dialog(props: &ShipsDialogProps) -> Html {
    let t = use_translator();
    let table_style = css!(
        r#"
        border-spacing: 1rem;
		text-align: left;
		width: 100%;
		"#
    );

    let sprite_style = css!(
        r#"
        display: inline-block;
		margin: 0.5em;
        "#
    );

    let select_factory = {
        let navigator = use_navigator().unwrap();
        move |entity_type| {
            let navigator = navigator.clone();
            Callback::from(move |_: MouseEvent| {
                navigator.push(&Mk48Route::ShipsSelected {
                    selected: PathParam(entity_type),
                });
            })
        }
    };

    html! {
        <NexusDialog title={translate!(t, "Ships")}>
            if let Some(selected) = props.selected {
                {entity_card(&t, &table_style, selected, None, None)}
                <p>{translate!(t, "Note that certain values are approximate and may be affected by other factors. For example, weapon damage depends on hit location.")}</p>
            } else {
                {(1..=EntityData::MAX_BOAT_LEVEL).map(move |level| html_nested!{
                    <div style="text-align: center;">
                        <h3>{format!("{} ({})", t.level(level as u32), t.score(level_to_score(level)))}</h3>
                        {EntityType::iter().filter(move |entity_type| entity_type.data().kind == EntityKind::Boat && entity_type.data().level == level).map(|entity_type| {
                            html_nested! {
                                <Sprite
                                    {entity_type}
                                    class={sprite_style.clone()}
                                    onclick={select_factory(entity_type)}
                                />
                            }
                        }).collect::<Html>()}
                    </div>
                }).collect::<Html>()}
            }
        </NexusDialog>
    }
}

fn entity_card(
    t: &Translator,
    table_style: &StyleSource,
    entity_type: EntityType,
    count: Option<u8>,
    reload_override: Option<Ticks>,
) -> Html {
    let data: &'static EntityData = entity_type.data();
    html! {
        <table class={table_style.clone()}>
            <tr>
                <td>
                    <h3>
                        {data.label}
                        if let Some(count) = count {
                            {format!(" × {count}")}
                        }
                    </h3>
                    <i>
                        if data.kind == EntityKind::Boat {
                            {t.level(data.level as u32)}
                        }
                        {" "}
                        {t.entity_kind_name(data.kind, data.sub_kind)}
                    </i>
                    if let Some(href) = data.link.clone() {
                        {" ("} <Link {href}>{"Learn more"}</Link>{")"}
                    }
                </td>
                <td rowspan="2">
                    <ul>
                        if data.length != 0.0 {
                            <li>{format!("{}: {:.1}m", translate!(t, "Length"), data.length)}</li>
                        }
                        if data.draft != Altitude::ZERO {
                            <li>{format!("{}: {:.1}m", translate!(t, "Draft"), data.draft.to_meters())}</li>
                        }
                        if data.speed != Velocity::ZERO {
                            <li>{format!("{}: {:.1}m/s ({:.1}kn)", translate!(t, "Speed"), data.speed.to_mps(), data.speed.to_knots())}</li>
                        }
                        if data.range != 0.0 {
                            <li>{format!("{}: {}m", translate!(t, "Range"), data.range as u32)}</li>
                        }
                        if data.depth != Altitude::ZERO {
                            <li>{format!("{}: {}m", translate!(t, "Max Depth"), data.depth.to_meters() as u16)}</li>
                        }
                        if data.lifespan != Ticks::ZERO {
                            <li>{format!("{}: {:.1}s", translate!(t, "Lifespan"), data.lifespan.to_secs())}</li>
                        }
                        if reload_override.unwrap_or(data.reload) != Ticks::ZERO {
                            <li>{format!("{}: {:.1}s", translate!(t, "Reload"), reload_override.unwrap_or(data.reload).to_secs())}</li>
                        }
                        if data.damage != 0.0 {
                            <li>{format!("{}: {:.2}", if data.kind == EntityKind::Boat { translate!(t, "Health") } else { translate!(t, "Damage") }, data.damage)}</li>
                        }
                        if data.anti_aircraft != 0.0 {
                            <li>{format!("{}: {:.2}", translate!(t, "Anti-Aircraft"), data.anti_aircraft)}</li>
                        }
                        if data.torpedo_resistance != 0.0 {
                            <li>{format!("{}: {}%", translate!(t, "Torpedo Resistance"), (data.torpedo_resistance * 100.0) as u16)}</li>
                        }
                        if data.stealth != 0.0 {
                            <li>{format!("{}: {}%", translate!(t, "Stealth"), (data.stealth * 100.0) as u16)}</li>
                        }
                        if data.npc {
                            <li>{translate!(t, "NPC only")}</li>
                        }
                    </ul>
                </td>
            </tr>
            <tr>
                <td>
                    <Sprite {entity_type}/>
                </td>
            </tr>
            {group_armaments(&data.armaments, &[]).into_iter().map(|Group{entity_type, total, reload_override, ..}| html_nested!{
                <tr>
                    <td colspan="2">
                        {entity_card(t, table_style, entity_type, Some(total), reload_override)}
                    </td>
                </tr>
            }).collect::<Html>()}
        </table>
    }
}
