use crate::dialog::dialog::Dialog;
use crate::translation::{t, Translation};
use yew::{function_component, html};

#[function_component(SettingsDialog)]
pub fn settings_dialog() -> Html {
    html! {
        <Dialog title={t().settings_title()}>{"Settings are forthcoming..."}</Dialog>
    }
}
