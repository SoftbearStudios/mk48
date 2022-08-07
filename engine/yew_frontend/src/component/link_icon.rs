use web_sys::{window, MouseEvent};
use yew::virtual_dom::AttrValue;
use yew::{function_component, html, Callback, Properties};
use yew_icons::{Icon, IconId};

#[derive(PartialEq, Properties)]
pub struct LinkIconProps {
    pub icon_id: IconId,
    pub title: Option<AttrValue>,
    pub link: AttrValue,
    #[prop_or("2.5rem".into())]
    pub size: AttrValue,
}

#[function_component(LinkIcon)]
pub fn link_icon(props: &LinkIconProps) -> Html {
    let onclick = {
        let link = props.link.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            if let Err(e) = window().unwrap().open_with_url_and_target(&link, "_blank") {
                client_util::console_log!("could not open link: {:?}", e);
            }
        })
    };

    html! {
        <Icon icon_id={props.icon_id} title={props.title.clone()} {onclick} width={props.size.clone()} height={props.size.clone()} style={"color: white; cursor: pointer; user-select: none;"}/>
    }
}
