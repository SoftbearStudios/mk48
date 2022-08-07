use web_sys::MouseEvent;
use yew::{function_component, html, Callback, Children, Properties};
use yew_router::history::History;
use yew_router::hooks::use_history;
use yew_router::Routable;

#[derive(PartialEq, Properties)]
pub struct RouteLinkProps<R: Routable> {
    pub children: Children,
    pub route: R,
}

#[function_component(RouteLink)]
pub fn route_link<R: Routable + Copy + 'static>(props: &RouteLinkProps<R>) -> Html {
    let onclick = {
        let route = props.route;
        let navigator = use_history().unwrap();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            e.stop_propagation();

            navigator.push(route);
        })
    };

    // Trick yew into not warning about bad practice.
    let href: &'static str = "javascript:void(0)";

    html! {
        <a {href} {onclick} style={"color: white; cursor: pointer; user-select: none;"}>
            {props.children.clone()}
        </a>
    }
}
