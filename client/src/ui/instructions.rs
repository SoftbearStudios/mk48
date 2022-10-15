use crate::translation::Mk48Translation;
use stylist::yew::styled_component;
use yew::{html, Properties};
use yew_frontend::translation::t;

#[derive(Copy, Clone, PartialEq, Properties)]
pub struct InstructionsProps {
    pub touch: bool,
    pub basics: bool,
    pub zoom: bool,
}

#[styled_component(Instructions)]
pub fn instructions(props: &InstructionsProps) -> Html {
    let div_style = css!(
        r#"
        pointer-events: none;
        user-select: none;
        color: white;
        "#
    );

    let t = t();

    html! {
        <div class={div_style}>
            if props.basics {
                <h2>{if props.touch { t.instruction_zoom_touch() } else { t.instruction_basics_mouse() }}</h2>
            }
            <div>
                if props.zoom {
                    <p>{if props.touch { t.instruction_zoom_touch() } else { t.instruction_basics_touch() }}</p>
                }
            </div>
        </div>
    }
}
