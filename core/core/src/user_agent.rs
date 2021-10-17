// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::id::*;
use lazy_static::lazy_static;
use log::error;
use servutil::user_agent::UserAgent;
use std::include_str;
use user_agent_parser::UserAgentParser;

lazy_static! {
    static ref USER_AGENT_PARSER: Option<UserAgentParser> =
        UserAgentParser::from_str(include_str!("regexes.yaml")).ok();
}

/// Bucketize user agent in order to limit the number of categories.
pub fn parse_user_agent(maybe_user_agent: Option<UserAgent>) -> Option<UserAgentId> {
    let mut user_agent_id = None;
    if let Some(user_agent) = maybe_user_agent {
        if let Some(ua_parser) = USER_AGENT_PARSER.as_ref() {
            let maybe_device_name = ua_parser.parse_device(&user_agent.0).name;
            if let Some(ref name) = maybe_device_name {
                match name.as_ref() {
                    "Spider" => user_agent_id = Some(UserAgentId::Spider),
                    _ => {}
                }
            }

            if user_agent_id.is_none() {
                if let Some(name) = ua_parser.parse_os(&user_agent.0).name {
                    user_agent_id = match name.as_ref() {
                        "Android" => Some(UserAgentId::Mobile),
                        "Chrome OS" => Some(UserAgentId::ChromeOS),
                        "iOS" => {
                            if let Some(name) = maybe_device_name {
                                match name.as_ref() {
                                    "iPad" => Some(UserAgentId::Tablet),
                                    _ => Some(UserAgentId::Mobile),
                                }
                            } else {
                                Some(UserAgentId::Mobile)
                            }
                        }
                        "Linux" | "Ubuntu" | "Mac OS X" | "Windows" => {
                            let mut result = Some(UserAgentId::Desktop);
                            if let Some(name) = ua_parser.parse_engine(&user_agent.0).name {
                                match name.as_ref() {
                                    "Blink" => result = Some(UserAgentId::DesktopChrome),
                                    "WebKit" => result = Some(UserAgentId::DesktopSafari),
                                    "Gecko" => result = Some(UserAgentId::DesktopFirefox),
                                    _ => {
                                        // "EdgeHTML" => pre-v79 Edge
                                        // "Presto" => pre-v15 Opera
                                        // "Trident" => IE
                                    }
                                }
                            }

                            result
                        }
                        _ => None,
                    };
                }
            }
        } else {
            error!("cannot identify user agent because regexes.yaml did not compile.");
        }
    }

    user_agent_id
}
