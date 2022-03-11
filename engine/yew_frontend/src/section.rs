use yew::{function_component, html, use_state, Callback, Children, Html, Properties};

#[derive(PartialEq, Properties)]
pub struct SectionProps {
    pub name: String,
    #[prop_or("left")]
    pub header_align: &'static str,
    #[prop_or(true)]
    pub start_open: bool,
    pub children: Children,
}

#[function_component(Section)]
pub fn section(props: &SectionProps) -> Html {
    let open = use_state(|| props.start_open);

    let onclick = {
        let open = open.clone();
        Callback::from(move |_| open.set(!*open))
    };

    html! {
        <>
            <h2
                {onclick}
                style={format!("text-align: {};", props.header_align)}
            >{&props.name}</h2>
            {
                if *open {
                    html! {
                        <div style="position: absolute; top: 0; left: 0;">
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
