// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::Mk48Phrases;
use crate::{
    armament::{group_armaments, Group},
    game::Mk48Game,
    ui::{sprite::Sprite, UiEvent, UiStatusPlaying},
};
use common::{
    altitude::Altitude,
    entity::{EntityData, EntitySubKind, EntityType},
};
use kodiak_client::glam::Vec2;
use kodiak_client::{use_translator, use_ui_event_callback, Translator};
use stylist::{yew::styled_component, StyleSource};
use web_sys::MouseEvent;
use yew::{classes, html, html_nested, Callback, Html, Properties};

#[derive(Properties, PartialEq)]
pub struct StatusProps {
    #[prop_or(None)]
    pub fps: Option<f32>,
    pub status: UiStatusPlaying,
}

#[styled_component(StatusOverlay)]
pub fn status_overlay(props: &StatusProps) -> Html {
    let button_style = css!(
        r#"
        color: white;
		padding: 0.5rem;
		filter: brightness(0.8);
		user-select: none;
		cursor: pointer;
        min-width: 5rem;

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
        padding: 0.5rem;
        "#
    );

    let consumption_style = css!(
        r#"
        float: right;
		color: white;
    "#
    );

    let consumed_style = css!(
        r#"
        opacity: 0.6;
        "#
    );

    let armaments_style = css!(
        r#"
        margin: auto;
        display: flex;
        flex-direction: row;
        width: min-content;

        @media (max-width: 700px) {
            flex-direction: column;
            width: max-content;
        }
        "#
    );

    let sprite_scale = css!(
        r#"
        @media (max-width: 1400px) {
            zoom: 0.75;
        }

        @media (max-width: 1100px) {
            zoom: 0.5;
        }

        @media (max-width: 800px) {
            zoom: 0.4;
        }
        "#
    );

    let t = use_translator();
    let status = &props.status;
    let data: &'static EntityData = props.status.entity_type.data();
    let ui_event_callback = use_ui_event_callback::<Mk48Game>();
    let select_factory = {
        let ui_event_callback = ui_event_callback.clone();
        move |entity_type: EntityType| {
            (props.status.armament != Some(entity_type)).then(move || {
                ui_event_callback.reform(move |_: MouseEvent| UiEvent::Armament(Some(entity_type)))
            })
        }
    };

    html! {<>
        <p style="margin: 0; margin-bottom: 0.5rem; font-family: monospace, sans-serif;">
            {data.label.replace(" ", "\u{00A0}")}
            {" "}
            {format!("{:\u{00A0}>4.1}kn", status.velocity.to_knots())}
            {" "}
            {format!("{:\u{00A0}>3}°\u{00A0}{:\u{00A0}<4}", status.direction.to_bearing(), format!("[{}]", status.direction.to_cardinal()))}
            {" "}
            {fmt_position(status.position)}
            if let Some(fps) = props.fps {
                {" "}
                {format!("{:\u{00A0}>5.1}\u{00A0}fps", fps)}
            }
        </p>
        <div class={armaments_style}>
            {group_armaments(&status.entity_type.data().armaments, &*status.armament_consumption).into_iter().map(|Group{entity_type, total, ready, ..}| {
                let onclick = select_factory.clone()(entity_type);
                html_nested!{
                    <div
                        class={classes!(
                            button_style.clone(),
                            onclick.is_none().then(|| button_selected_style.clone())
                        )}
                        {onclick}
                    >
                        <Sprite
                            {entity_type}
                            class={classes!(
                                (ready == 0).then(|| consumed_style.clone()),
                                sprite_scale.clone()
                            )}
                        />
                        <span class={consumption_style.clone()}>{format!("{ready}/{total}")}</span>
                    </div>
                }
            }).collect::<Html>()}
            {surface_button(
                &t,
                props.status.entity_type,
                props.status.submerge,
                &button_style,
                &button_selected_style,
                &ui_event_callback
            )}
            {active_sensor_button(&t,
                props.status.entity_type,
                props.status.active,
                props.status.altitude,
                &button_style,
                &button_selected_style,
                &ui_event_callback
            )}
        </div>
    </>}
}

fn fmt_position(position: Vec2) -> String {
    fn fmt_coordinate(coordinate: f32, positive: char, negative: char) -> String {
        format!(
            "{}{}",
            coordinate.round().abs(),
            if coordinate >= 0.0 {
                positive
            } else {
                negative
            }
        )
    }
    format!(
        "{:\u{00A0}>6},\u{00A0}{:\u{00A0}>5})",
        "(".to_owned() + &fmt_coordinate(position.x, 'E', 'W'),
        fmt_coordinate(position.y, 'N', 'S')
    )
}

fn surface_button(
    t: &Translator,
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
    t: &Translator,
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
            .intersperse(" / ".to_string())
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
