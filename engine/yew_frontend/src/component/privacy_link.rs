// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::component::route_link::RouteLink;
use crate::translation::{t, Translation};
use crate::Route;
use yew::{function_component, html};

#[function_component(PrivacyLink)]
pub fn privacy_link() -> Html {
    html! {
        <RouteLink<Route> route={Route::Privacy}>{t().privacy_hint()}</RouteLink<Route>>
    }
}
