// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::curtain::Curtain;
use crate::component::positioner::{Position, Positioner};
use crate::component::spinner::Spinner;
use crate::translation::{use_translation, Translation};
use stylist::yew::styled_component;
use yew::{html, Html};

#[styled_component(Reconnecting)]
pub fn reconnecting() -> Html {
    let message = use_translation().connection_losing_message();
    html! {
        <Curtain>
            <Positioner position={Position::Center}>
                <Spinner/>
                <p>{message}</p>
            </Positioner>
        </Curtain>
    }
}
