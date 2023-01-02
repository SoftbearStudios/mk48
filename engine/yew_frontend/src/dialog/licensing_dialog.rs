// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::dialog::dialog::Dialog;
use crate::frontend::use_ctw;
use yew::{function_component, html, html_nested, Html};

#[function_component(LicensingDialog)]
pub fn licensing_dialog() -> Html {
    let licenses = use_ctw().licenses;

    html! {
        <Dialog title={"Licensing"}>
            <h2>{"Open Source Software"}</h2>
            <p>{"The game would not exist without free and open source software."}</p>
            {licensing(licenses)}
        </Dialog>
    }
}

pub fn licensing(licenses: &'static [(&'static str, &'static [&'static str])]) -> Html {
    licenses
        .iter()
        .map(|(license, names)| {
            html! {
                <>
                    <h3>{license}</h3>
                    <ul>
                        {names.iter().map(|name| html_nested!{
                            <li>{name}</li>
                        }).collect::<Html>()}
                    </ul>
                </>
            }
        })
        .collect::<Html>()
}
