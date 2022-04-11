// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use stylist::yew::styled_component;
use web_sys::MouseEvent;
use yew::{html, use_state, Callback, Children, Properties};
use yew_icons::{Icon, IconId};

#[derive(PartialEq, Properties)]
pub struct SectionProps {
    pub children: Children,
    #[prop_or(false)]
    pub closable: bool,
    #[prop_or(None)]
    pub header_align: Option<TextAlign>,
    pub name: String,
    #[prop_or(None)]
    pub on_left_arrow: Option<Callback<()>>,
    #[prop_or(None)]
    pub on_right_arrow: Option<Callback<()>>,
    #[prop_or(false)]
    pub right_arrow: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Right,
}

#[styled_component(Section)]
pub fn section(props: &SectionProps) -> Html {
    let open = use_state(|| true);

    let onclick = if !props.closable {
        None
    } else {
        let open = open.clone();
        Some(Callback::from(move |_| open.set(!*open)))
    };

    let div_css_class = css!(
        r#"
        left: 0;
        position: absolute;
        top: 0;
    "#
    );

    let h2_css_class = css!(
        r#"
        h2 {
            color: white;
            cursor: pointer;
            font-weight: bold;
            margin: 0;
            user-select: none;
            text-align: center;
            transition: filter 0.1s;
        }

        h2:hover {
            filter: opacity(0.85);
        }

        @media (min-width: 1000px) {
            h2 {
                white-space: nowrap;
            }
        }
    "#
    );

    let h2_style = props.header_align.map(|header_align| match header_align {
        TextAlign::Left => "left",
        TextAlign::Right => "right",
    });

    let span_css_class = css!(
        r#"
        .disable {
            opacity: 0;
        }
    "#
    );

    let left_click = props.on_left_arrow.as_ref().map(|cb| {
        cb.reform(|e: MouseEvent| {
            e.stop_propagation();
        })
    });

    let right_click = props.on_right_arrow.as_ref().map(|cb| {
        cb.reform(|e: MouseEvent| {
            e.stop_propagation();
        })
    });

    html! {
        <>
            <h2
                class={h2_css_class}
                style={h2_style}
                {onclick}
                >
                if left_click.is_some() {
                    <span class={span_css_class.clone()} onclick={left_click}>
                        <Icon icon_id={IconId::FontAwesomeSolidSquareCaretLeft}/>
                    </span>
                }
                {&props.name}
                if right_click.is_some() {
                    <span class={span_css_class} onclick={right_click}>
                        <Icon icon_id={IconId::FontAwesomeSolidSquareCaretRight}/>
                    </span>
                }
            </h2>
            {
                if *open {
                    html! {
                        <div class={div_css_class}>
                            {props.children.clone()}
                        </div>
                    }
                } else {
                    html! {}
                }
            }
        </>
    }
}
