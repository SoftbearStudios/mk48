// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::phrases::Mk48Phrases;
use common::entity::{EntityData, EntityType};
use kodiak_client::use_translator;
use lazy_static::lazy_static;
use sprite_sheet::SpriteSheet;
use stylist::yew::styled_component;
use web_sys::MouseEvent;
use yew::virtual_dom::AttrValue;
use yew::{classes, html, Callback, Children, Classes, Html, Properties};

#[derive(Properties, PartialEq)]
pub struct SpriteProps {
    pub entity_type: EntityType,
    #[prop_or(None)]
    pub title: Option<AttrValue>,
    #[prop_or(None)]
    pub onclick: Option<Callback<MouseEvent>>,
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub image_class: Classes,
    #[prop_or(None)]
    pub children: Option<Children>,
}

lazy_static! {
    static ref SPRITE_SHEET: SpriteSheet =
        serde_json::from_str(include_str!("./sprites_css.json")).unwrap();
}

#[styled_component(Sprite)]
pub fn sprite(props: &SpriteProps) -> Html {
    let container_style = css!(
        r#"
        position: relative;
        text-align: center;
        display: inline-block;
        "#
    );

    let image_style = css!(
        r#"
        background-image: url("/data/sprites_css.png");
        position: absolute;
        "#
    );

    let children_style = css!(
        r#"
        margin-top: 0.25rem;
        opacity: 0.8;
        "#
    );

    let clickable_style = css!(
        r#"
        cursor: pointer;
        "#
    );

    let t = use_translator();
    let data: &'static EntityData = props.entity_type.data();
    let sprite = SPRITE_SHEET
        .sprites
        .get(props.entity_type.as_str())
        .expect(&format!("should have sprite for {:?}", props.entity_type));
    let title = props.title.clone().unwrap_or_else(|| {
        format!(
            "{} ({})",
            data.label,
            t.entity_kind_name(data.kind, data.sub_kind)
        )
        .into()
    });

    html! {
        <div
            onclick={props.onclick.clone()}
            class={classes!(
                container_style,
                props.class.clone(),
                props.onclick.is_some().then_some(clickable_style)
            )}
            style={format!("width: {}px; height: {}px;", sprite.width, sprite.height)}
        >
            <div {title} class={classes!(image_style, props.image_class.clone())} style={format!("background-position: -{}px -{}px; width: {}px; height: {}px;", sprite.x, sprite.y, sprite.width, sprite.height)}></div>
            if let Some(children) = props.children.clone() {
                <div class={children_style}>
                    {children}
                </div>
            }
        </div>
    }
}
