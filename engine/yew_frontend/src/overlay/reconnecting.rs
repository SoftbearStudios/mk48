// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::curtain::Curtain;
use crate::component::positioner::{Position, Positioner};
use crate::component::spinner::Spinner;
use stylist::yew::styled_component;
use yew::html;

#[styled_component(Reconnecting)]
pub fn reconnecting() -> Html {
    html! {
        <Curtain>
            <Positioner position={Position::Center}>
                <Spinner/>
                <p>{"Connection lost, attempting to reconnect..."}</p>
            </Positioner>
        </Curtain>
    }
}
