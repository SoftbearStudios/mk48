// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::id::InvitationId;
use core_protocol::name::Referrer;
use std::num::NonZeroU32;
use std::str::FromStr;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, Window};
//use wasm_bindgen::prelude::{Closure};

/// Gets the canvas to use for WebGL.
pub fn canvas() -> Result<HtmlCanvasElement, String> {
    let document = window()?.document().ok_or("no document".to_string())?;
    document
        .get_element_by_id("canvas")
        .ok_or("no canvas".to_string())?
        .dyn_into::<HtmlCanvasElement>()
        .ok()
        .ok_or("invalid canvas".to_string())
}

/// Gets the current domain name e.g. mk48.io
pub fn domain_name() -> String {
    domain_name_of(host())
}

/// Gets the domain name component of a host string e.g. mk48.io
pub fn domain_name_of(host: String) -> String {
    let mut split: Vec<_> = host.split('.').collect();
    if split.len() > 2 {
        let tld = split.pop().unwrap();
        let domain = split.pop().unwrap();
        domain.to_owned() + "." + tld
    } else {
        host
    }
}

/// e.g. foo.mk48.io
/// This is a problematic API, since it won't handle redirects.
pub fn host() -> String {
    web_sys::window().unwrap().location().host().unwrap()
}

/// Reads the `InvitationId` present in the path, if any.
/// Path should resemble /#/invite/INVITE_CODE_HERE
pub fn invitation_id() -> Option<InvitationId> {
    web_sys::window()
        .unwrap()
        .location()
        .hash()
        .ok()
        .filter(|h| h.contains("/invite/"))
        .and_then(|h| {
            h.split('/')
                .last()
                .and_then(|n| NonZeroU32::from_str(n).ok())
        })
        .map(InvitationId)
}

/* TODO (for now, this is done via JS)
pub fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) {
    window()?
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
} */

/// Gets the HTTP referrer.
pub fn referrer() -> Option<Referrer> {
    Referrer::new(&web_sys::window().unwrap().document().unwrap().referrer())
}

/// Gets the window.
fn window() -> Result<Window, String> {
    web_sys::window().ok_or("no window".to_string())
}

/// Gets the string, ws or wss, for the websocket protocol to use.
/// This is a problematic API because it does not respect redirect schemes.
pub fn ws_protocol() -> &'static str {
    if web_sys::window().unwrap().location().protocol().unwrap() == "http:" {
        "ws"
    } else {
        "wss"
    }
}
