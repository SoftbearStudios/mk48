// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::id::*;
use lazy_static::lazy_static;
use servutil::user_agent::UserAgent;
use uaparser::Parser;
use uaparser::UserAgentParser;

lazy_static! {
    pub static ref USER_AGENT_PARSER: UserAgentParser =
        UserAgentParser::from_bytes(include_bytes!("regexes.yaml"))
            .expect("regexes.yaml did not compile");
}

/// Bucketize user agent in order to limit the number of categories.
pub fn parse_user_agent(maybe_user_agent: Option<UserAgent>) -> Option<UserAgentId> {
    if let Some(UserAgent(user_agent)) = maybe_user_agent.as_ref() {
        let ua_parser: &UserAgentParser = &USER_AGENT_PARSER;
        let client = ua_parser.parse(user_agent);
        // println!("{:?}", client);

        let device = client.device.family.as_str();
        match device {
            "Spider" => Some(UserAgentId::Spider),
            _ => {
                let os = client.os.family;
                match os.as_ref() {
                    "Android" => Some(UserAgentId::Mobile),
                    "Chrome OS" => Some(UserAgentId::ChromeOS),
                    "iOS" => match device {
                        "iPad" => Some(UserAgentId::Tablet),
                        _ => Some(UserAgentId::Mobile),
                    },
                    "Linux" | "Ubuntu" | "Mac OS X" | "Windows" => {
                        match client.user_agent.family.as_ref() {
                            "Chrome" => Some(UserAgentId::DesktopChrome),
                            "Safari" => Some(UserAgentId::DesktopSafari),
                            "Firefox" => Some(UserAgentId::DesktopFirefox),
                            _ => {
                                // "EdgeHTML" => pre-v79 Edge
                                // "Presto" => pre-v15 Opera
                                // "Trident" => IE
                                // println!("unkown family {}", client.user_agent.family);
                                Some(UserAgentId::Desktop)
                            }
                        }
                    }
                    _ => None,
                }
            }
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use arrayvec::ArrayString;
    use core_protocol::id::*;
    use servutil::user_agent::UserAgent;

    #[test]
    fn test_parse_user_agent() {
        let tests = [
            ("Mozilla/5.0 (Macintosh; Intel Mac OS X 10.14; rv:81.0) Gecko/20100101 Firefox/81.0", UserAgentId::DesktopFirefox),
            ("Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/51.0.2704.103 Safari/537.36", UserAgentId::DesktopChrome),
            ("Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)", UserAgentId::Spider),
            ("Mozilla/5.0 (Linux; U; Android 4.4.2; en-US; HMNOTE 1W Build/KOT49H) AppleWebKit/534.30 (KHTML, like Gecko) Version/4.0 UCBrowser/11.0.5.850 U3/0.8.0 Mobile Safari/534.30", UserAgentId::Mobile),
            // ("Mozilla/5.0 (Linux; Android 8.1; EML-L29 Build/HUAWEIEML-L29; xx-xx) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/65.0.3325.109 Mobile Safari/537.36 (iPad; iPhone; CPU iPhone OS 13_2_3 like Mac OS X)", UserAgentId::Tablet)
        ];

        for (user_agent, correct_id) in tests {
            let user_agent = UserAgent(ArrayString::from(user_agent).unwrap());
            let parsed_id = super::parse_user_agent(Some(user_agent));
            assert_eq!(parsed_id, Some(correct_id));
        }
    }
}
