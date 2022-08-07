// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use arrayvec::ArrayString;
use core_protocol::id::UserAgentId;
use core_protocol::name::trim_and_slice_up_to_array_string;
use serde::{Deserialize, Serialize};
use woothee::parser::Parser;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct UserAgent(pub ArrayString<384>);

impl UserAgent {
    pub fn new(str: &str) -> Self {
        Self(trim_and_slice_up_to_array_string(str))
    }

    /// Bucketize user agent in order to limit the number of categories.
    pub fn into_id(self) -> Option<UserAgentId> {
        // The best resource for understanding woothee output is https://github.com/woothee/woothee-rust/tree/master/tests
        // Parser::new() is currently a no-op, so it is fine to do it for every parsing job.
        Parser::new()
            .parse(&self.0)
            .and_then(|res| match res.category {
                "crawler" => Some(UserAgentId::Spider),
                "pc" => Some(match res.name {
                    "Chrome" => match res.os {
                        "ChromeOS" => UserAgentId::ChromeOS,
                        _ => UserAgentId::DesktopChrome,
                    },
                    "Firefox" => UserAgentId::DesktopFirefox,
                    "Safari" => UserAgentId::DesktopSafari,
                    _ => UserAgentId::Desktop,
                }),
                "mobilephone" | "smartphone" => Some(match res.os {
                    "iPad" => UserAgentId::Tablet,
                    _ => UserAgentId::Mobile,
                }),
                _ => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::user_agent::UserAgent;
    use arrayvec::ArrayString;
    use core_protocol::id::*;

    #[test]
    fn test_parse_user_agent() {
        let tests = [
            ("Mozilla/5.0 (Macintosh; Intel Mac OS X 10.14; rv:81.0) Gecko/20100101 Firefox/81.0", UserAgentId::DesktopFirefox),
            ("Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/51.0.2704.103 Safari/537.36", UserAgentId::DesktopChrome),
            ("Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)", UserAgentId::Spider),
            ("Mozilla/5.0 (Linux; U; Android 4.4.2; en-US; HMNOTE 1W Build/KOT49H) AppleWebKit/534.30 (KHTML, like Gecko) Version/4.0 UCBrowser/11.0.5.850 U3/0.8.0 Mobile Safari/534.30", UserAgentId::Mobile),
            //("Mozilla/5.0 (Linux; Android 8.1; EML-L29 Build/HUAWEIEML-L29; xx-xx) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/65.0.3325.109 Mobile Safari/537.36 (iPad; iPhone; CPU iPhone OS 13_2_3 like Mac OS X)", UserAgentId::Tablet)
        ];

        for (user_agent, correct_id) in tests {
            let user_agent = UserAgent(ArrayString::from(user_agent).unwrap());
            let parsed_id = user_agent.into_id();
            assert_eq!(parsed_id, Some(correct_id));
        }
    }
}
