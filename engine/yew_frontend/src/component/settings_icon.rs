use crate::component::route_icon::RouteIcon;
use crate::translation::{t, Translation};
use crate::Route;
use yew::virtual_dom::AttrValue;
use yew::{function_component, html, Properties};
use yew_icons::IconId;

#[derive(PartialEq, Properties)]
pub struct SettingsIconProps {
    #[prop_or("2rem".into())]
    pub size: AttrValue,
}

#[function_component(SettingsIcon)]
pub fn settings_icon(props: &SettingsIconProps) -> Html {
    html! {
        <RouteIcon<Route> icon_id={IconId::BootstrapGear} title={t().settings_hint()} route={Route::Settings} size={props.size.clone()}/>
    }
}
