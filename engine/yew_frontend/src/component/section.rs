// SPDX-FileCopyrightText: 2022 Softbear, Inc.

use stylist::yew::styled_component;
use web_sys::MouseEvent;
use yew::{html, use_state, Callback, Children, Properties};
use yew_icons::{Icon, IconId};

#[derive(PartialEq, Properties)]
pub struct SectionProps {
    pub children: Children,
    #[prop_or(true)]
    pub closable: bool,
    pub name: String,
    #[prop_or(None)]
    pub on_left_arrow: Option<Callback<()>>,
    #[prop_or(None)]
    pub on_right_arrow: Option<Callback<()>>,
    #[prop_or(false)]
    pub right_arrow: bool,
}

#[styled_component(Section)]
pub fn section(props: &SectionProps) -> Html {
    let open = use_state(|| true);

    let onclick = props.closable.then(|| {
        let open = open.clone();
        Callback::from(move |_| open.set(!*open))
    });

    let div_css_class = css!(r#""#);

    let h2_css_class = css!(
        r#"
        color: white;
        cursor: pointer;
        font-weight: bold;
        margin: 0;
        user-select: none;
        transition: filter 0.1s;

        :hover {
            filter: opacity(0.85);
        }
    "#
    );

    /*
       @media (min-width: 1000px) {
           h2 {
               white-space: nowrap;
           }
       }
    */

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

    const ICON_WIDTH: &'static str = "1.5rem";
    const ICON_HEIGHT: &'static str = "1.2rem";

    html! {
        <>
            <h2
                class={h2_css_class}
                {onclick}
                >
                if *open && left_click.is_some() {
                    <span class={span_css_class.clone()} onclick={left_click}>
                        <Icon icon_id={IconId::FontAwesomeSolidSquareCaretLeft} width={ICON_WIDTH.to_string()} height={ICON_HEIGHT.to_string()}/>
                    </span>
                }
                {&props.name}
                if *open && right_click.is_some() {
                    <span class={span_css_class} onclick={right_click}>
                        <Icon icon_id={IconId::FontAwesomeSolidSquareCaretRight} width={ICON_WIDTH.to_string()} height={ICON_HEIGHT.to_string()}/>
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
