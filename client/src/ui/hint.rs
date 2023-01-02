// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::translation::Mk48Translation;
use common::entity::EntityType;
use stylist::yew::styled_component;
use web_sys::HtmlDivElement;
use yew::{html, use_effect_with_deps, use_node_ref, Html, Properties};
use yew_frontend::translation::use_translation;

#[derive(PartialEq, Properties)]
pub struct HintProps {
    pub entity_type: EntityType,
}

#[styled_component(Hint)]
pub fn hint(props: &HintProps) -> Html {
    let hint_style = css!(
        r#"
        background-color: #00000040;
        color: white;
        height: min-content;
        left: 50%;
        margin: auto;
        padding: 0.5em;
        pointer-events: none;
        position: absolute;
        text-align: center;
        top: 65%;
        transform: translate(-50%, -50%);
        user-select: none;
        animation: fade 5.0s;
        animation-fill-mode: both;

        @keyframes fade {
            0% {
                opacity: 0.0;
            }
            10% {
                opacity: 0.9;
            }
            80% {
                opacity: 0.9;
            }
            100% {
                opacity: 0.0;
            }
        }
    "#
    );

    let container_ref = use_node_ref();

    {
        let container_ref = container_ref.clone();
        use_effect_with_deps(
            move |_| {
                if let Some(container) = container_ref.cast::<HtmlDivElement>() {
                    let style = container.style();
                    // Reset the animation.
                    let _ = style.set_property("animation", "none");
                    // Trigger a reflow.
                    let _ = container.offset_width();
                    let _ = style.remove_property("animation");
                }
                || {}
            },
            props.entity_type,
        );
    }

    let t = use_translation();
    let data = props.entity_type.data();
    html! {
        <div class={hint_style} ref={container_ref}>
            {t.entity_kind_hint(data.kind, data.sub_kind)}
        </div>
    }
}
