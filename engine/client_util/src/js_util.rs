// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::id::InvitationId;
use core_protocol::name::Referrer;
use js_hooks::{document, window};
use std::num::NonZeroU32;
use std::str::FromStr;

/// Gets the domain name component of a host string e.g. mk48.io
pub fn domain_name_of(host: &str) -> String {
    let mut split: Vec<_> = host.split('.').collect();
    if split.len() > 2 {
        let tld = split.pop().unwrap();
        let domain = split.pop().unwrap();
        domain.to_owned() + "." + tld
    } else {
        host.to_owned()
    }
}

/// e.g. foo.mk48.io
/// This is a problematic API, since it won't handle redirects.
pub fn host() -> String {
    window().location().host().unwrap()
}

/// Reads the `InvitationId` present in the path, if any.
/// Path should resemble /invite/INVITE_CODE_HERE
pub fn invitation_id() -> Option<InvitationId> {
    window()
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

/// Gets the HTTP referrer.
pub fn referrer() -> Option<Referrer> {
    Referrer::new(&document().referrer())
}

/// Returns `true` if the user agent is a mobile browser (may overlook some niche platforms).
pub fn is_mobile() -> bool {
    let user_agent = window().navigator().user_agent();
    user_agent
        .map(|user_agent| {
            ["iPhone", "iPad", "iPod", "Android"]
                .iter()
                .any(|platform| user_agent.contains(platform))
        })
        .unwrap_or(false)
}

/// Gets the string, ws or wss, for the websocket protocol to use.
/// This is a problematic API because it does not respect redirect schemes.
pub fn is_https() -> bool {
    window()
        .location()
        .protocol()
        .map(|p| p != "http:")
        .unwrap_or(true)
}

pub fn ws_protocol(encrypted: bool) -> &'static str {
    if encrypted {
        "wss"
    } else {
        "ws"
    }
}
