// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::armament::{group_armaments, Group};
use crate::translation::Mk48Translation;
use crate::ui::sprite::Sprite;
use crate::ui::{UiEvent, UiStatusPlaying};
use crate::Mk48Game;
use common::altitude::Altitude;
use common::entity::{EntityData, EntitySubKind, EntityType};
use core_protocol::id::LanguageId;
use stylist::yew::styled_component;
use stylist::{css, StyleSource};
use web_sys::MouseEvent;
use yew::{classes, html, html_nested, Callback, Html, Properties};
use yew_frontend::component::section::Section;
use yew_frontend::frontend::Gctw;
use yew_frontend::translation::t;

#[derive(Properties, PartialEq)]
pub struct ShipControlsProps {
    pub status: UiStatusPlaying,
}

#[styled_component(ShipControls)]
pub fn ship_controls(props: &ShipControlsProps) -> Html {
    let button_style = css!(
        r#"
        color: white;
		padding: 0.5em;
		filter: brightness(0.8);
		user-select: none;
		cursor: pointer;

		:hover {
            background-color: #44444440;
            filter: brightness(0.9);
        }
    "#
    );

    // !important to override the :hover.
    let button_selected_style = css!(
        r#"
        background-color: #44444480 !important;
        cursor: default;
        filter: brightness(1.2) !important;
        padding: 0.5em;
        "#
    );

    let consumption_style = css!(
        r#"
        float: right;
		color: white;
    "#
    );

    let data: &'static EntityData = props.status.entity_type.data();

    let ui_event_callback = &Gctw::<Mk48Game>::use_ui_event_callback();
    let select_factory = |entity_type: EntityType| {
        (!props.status.armament.contains(&entity_type)).then(move || {
            ui_event_callback.reform(move |_: MouseEvent| UiEvent::Armament(Some(entity_type)))
        })
    };

    let t = t();
    let status = &props.status;
    let ui_event_callback = Gctw::<Mk48Game>::use_ui_event_callback();
    html! {
        <Section name={data.label.clone()} closable={false}>
            {group_armaments(&status.entity_type.data().armaments, &*status.armament_consumption).into_iter().map(|Group{entity_type, total, ready}| {
                let onclick = select_factory(entity_type);
                html_nested!{
                    <div class={classes!(button_style.clone(), onclick.is_none().then(|| button_selected_style.clone()))} {onclick}>
                        <Sprite {entity_type}/>
                        <span class={consumption_style.clone()}>{format!("{ready}/{total}")}</span>
                    </div>
                }
            }).collect::<Html>()}
            {surface_button(t, props.status.entity_type, props.status.submerge, &button_style, &button_selected_style, &ui_event_callback)}
            {active_sensor_button(t, props.status.entity_type, props.status.active, props.status.altitude, &button_style, &button_selected_style, &ui_event_callback)}
        </Section>
    }
}

fn surface_button(
    t: LanguageId,
    entity_type: EntityType,
    submerge: bool,
    button_style: &StyleSource,
    button_selected_style: &StyleSource,
    ui_event_callback: &Callback<UiEvent>,
) -> Html {
    if entity_type.data().sub_kind != EntitySubKind::Submarine {
        Html::default()
    } else {
        let onclick = ui_event_callback.reform(move |_: MouseEvent| UiEvent::Submerge(!submerge));

        html! {
            <div class={classes!(button_style.clone(), (!submerge).then(|| button_selected_style.clone()))} {onclick} title={t.ship_surface_hint()}>
                {t.ship_surface_label()}
            </div>
        }
    }
}

fn active_sensor_button(
    t: LanguageId,
    entity_type: EntityType,
    active: bool,
    altitude: Altitude,
    button_style: &StyleSource,
    button_selected_style: &StyleSource,
    ui_event_callback: &Callback<UiEvent>,
) -> Html {
    let data: &'static EntityData = entity_type.data();
    let sensors = &data.sensors;
    if !(sensors.radar.range > 0.0 || sensors.sonar.range > 0.0) {
        Html::default()
    } else {
        let sensors = (sensors.radar.range >= 0.0 && !altitude.is_submerged())
            .then(|| t.sensor_radar_label())
            .into_iter()
            .chain(
                (sensors.sonar.range >= 0.0 && !altitude.is_airborne())
                    .then(|| t.sensor_sonar_label()),
            )
            .intersperse(" / ")
            .collect::<String>();
        let title = t.sensor_active_hint(&sensors);
        let onclick = ui_event_callback.reform(move |_: MouseEvent| UiEvent::Active(!active));

        html! {
            <div class={classes!(button_style.clone(), active.then(|| button_selected_style.clone()))} {onclick} {title}>
                {t.sensor_active_label()}
            </div>
        }
    }
}
