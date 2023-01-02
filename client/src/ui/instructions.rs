use crate::translation::Mk48Translation;
use stylist::yew::styled_component;
use yew::{html, Html, Properties};
use yew_frontend::component::positioner::Position;
use yew_frontend::translation::use_translation;

#[derive(Copy, Clone, PartialEq, Properties)]
pub struct InstructionsProps {
    pub position: Position,
    pub status: InstructionStatus,
}

#[derive(Copy, Clone, Default, PartialEq)]
pub struct InstructionStatus {
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

    let p_style = css!(
        r#"
        font-size: 1.25rem;
        "#
    );

    let t = use_translation();

    html! {
        <div id="instructions" class={div_style} style={props.position.to_string()}>
            if props.status.basics {
                <h2>{if props.status.touch { t.instruction_basics_touch() } else { t.instruction_basics_mouse() }}</h2>
            }
            <div>
                if props.status.zoom {
                    <p class={p_style}>{if props.status.touch { t.instruction_zoom_touch() } else { t.instruction_zoom_mouse() }}</p>
                }
            </div>
        </div>
    }
}
