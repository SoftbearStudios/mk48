// SPDX-FileCopyrightText: 2022 Softbear, Inc.

pub fn set_location_href(href: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_href(href);
    }
}
