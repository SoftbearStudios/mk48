// SPDX-FileCopyrightText: 2024 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::ship_menu::ShipMenu;
use crate::ui::{UiEvent, UiStatusPlaying};
use crate::Mk48Game;
use common::entity::EntityData;
use common::util::level_to_score;
use kodiak_client::{
    map_ranges, use_translator, use_ui_event_callback, Instructions, Meter, Position,
};
use stylist::yew::styled_component;
use yew::{html, Html, Properties};

#[derive(Properties, PartialEq)]
pub struct UpgradeOverlayProps {
    pub position: Position,
    pub score: u32,
    pub status: UiStatusPlaying,
}

#[styled_component(UpgradeOverlay)]
pub fn upgrade_overlay(props: &UpgradeOverlayProps) -> Html {
    let div_style = css!(
        r#"
        pointer-events: none;
        user-select: none;
        color: white;
        min-width: 30%;
        "#
    );

    let meter_style = css!(
        r#"
        border-top-left-radius: 0 !important;
        border-top-right-radius: 0 !important;
        border-top-width: 0 !important;
        "#
    );

    let t = use_translator();
    let onclick = use_ui_event_callback::<Mk48Game>().reform(UiEvent::Upgrade);
    let level = props.status.entity_type.data().level;
    let next_level = level + 1;
    let level_score = level_to_score(level);
    let next_level_score = level_to_score(next_level);
    let progress = map_ranges(
        props.score as f32,
        // Prevent rounding down at 100%.
        level_score as f32..next_level_score as f32 - 0.1,
        0.0..1.0,
        true,
    );
    html! {
        <ShipMenu
            entity={Some((props.status.entity_type, props.status.position))}
            score={props.score}
            position={props.position.clone()}
            {onclick}
        >
            <div
                id="instructions_meter"
                class={div_style}
                style={Position::TopMiddle { margin: "0" }.to_string()}
            >
                if next_level <= EntityData::MAX_BOAT_LEVEL {
                    <Meter
                        value={progress}
                        background_color={0x3e3333}
                        border_color={0x686868}
                        class={meter_style}
                    >{t.upgrade_to_level_progress((progress * 100.0) as u8, next_level as u32)}</Meter>
                    <Instructions primary={props.status.primary.clone()} secondary={props.status.secondary.clone()}/>
                }
            </div>
        </ShipMenu>
    }
}
