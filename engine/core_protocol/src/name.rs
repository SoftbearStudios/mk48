// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::id::PlayerId;
use arrayvec::ArrayString;
use glam::Vec3;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

/// An alias, e.g. "mrbig", is NOT a real name.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlayerAlias(ArrayString<12>);
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Referrer(pub ArrayString<16>);
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SurveyDetail(pub ArrayString<384>);
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TeamName(ArrayString<12>);

lazy_static! {
    static ref BOT_NAMES: Box<[&'static str]> = include_str!("./famous_bots.txt")
        .split('\n')
        .filter(|s| !s.is_empty() && s.len() <= PlayerAlias::capacity())
        .collect();
}

/// A player's alias (not their real name).
impl PlayerAlias {
    /// Converts the string into a valid alias, which is never empty when done on the server.
    pub fn new(str: &str) -> Self {
        #[cfg(feature = "server")]
        {
            let mut string = rustrict::Censor::from_str(str)
                .with_censor_first_character_threshold(rustrict::Type::INAPPROPRIATE)
                .censor();

            let trimmed = rustrict::trim_whitespace(&string);

            if trimmed.starts_with('[') && trimmed.contains(']') {
                // Prevent alias confused with team name.
                string = string.replace('[', "<").replace(']', ">");
            }

            let ret = Self(slice_up_to_array_string(&string));

            return if ret.0.is_empty() {
                Self::default()
            } else {
                ret
            };
        }

        #[cfg(not(feature = "server"))]
        Self(slice_up_to_array_string(str))
    }

    pub fn from_bot_player_id(player_id: PlayerId) -> Self {
        //debug_assert!(player_id.is_bot());
        let names = &BOT_NAMES;
        Self::new(names[player_id.0.get() as usize % names.len()])
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn capacity() -> usize {
        Self(ArrayString::new()).0.capacity()
    }
}

impl Default for PlayerAlias {
    fn default() -> Self {
        Self(ArrayString::from("Guest").unwrap())
    }
}

impl Display for PlayerAlias {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Referrer {
    /// For example, given "https://foo.bar.com:1234/moo.zoo/woo.hoo" the referer will be "bar".
    pub fn new(raw: &str) -> Option<Self> {
        let a: Vec<&str> = raw.split("://").into_iter().collect();
        let b = if a.len() < 2 { raw } else { a[1] };

        let c: Vec<&str> = b.split("/").into_iter().collect();
        let d = if c.len() < 2 { b } else { c[0] };

        let e: Vec<&str> = d.split(".").into_iter().collect();
        let n = e.len();
        if n > 1 {
            let mut cooked = e[n - 2];
            if n > 2 && cooked == "com" {
                // e.g. "foo.com.uk"
                cooked = e[n - 3];
            }
            Some(Self(slice_up_to_array_string(cooked)))
        } else if n == 1 && !e[0].is_empty() {
            // e.g. localhost
            Some(Self(slice_up_to_array_string(e[0])))
        } else {
            None
        }
    }
}

impl TeamName {
    pub fn new(str: &str) -> Self {
        #[cfg(feature = "server")]
        let str = &rustrict::Censor::from_str(str)
            .with_censor_first_character_threshold(rustrict::Type::INAPPROPRIATE)
            .take(6)
            .collect::<String>();

        Self(slice_up_to_array_string(
            rustrict::trim_whitespace(str)
                .trim_start_matches('[')
                .trim_end_matches(']'),
        ))
    }

    /*
    /// Creates a form optimized for uniqueness checking.
    pub fn canonicalize(&self) -> Self {
        Self(self.0.to_lowercase())
    }
     */

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for TeamName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub fn slice_up_to(s: &str, bytes: usize) -> &str {
    let s = rustrict::trim_whitespace(s);
    let mut idx = bytes;
    while !s.is_char_boundary(idx) {
        idx -= 1;
    }
    &s[..idx]
}

pub fn slice_up_to_array_string<const CAPACITY: usize>(s: &str) -> ArrayString<CAPACITY> {
    ArrayString::from(slice_up_to(s, CAPACITY)).unwrap()
}

pub type Location = Vec3;

#[cfg(test)]
mod test {
    use crate::name::TeamName;

    #[test]
    fn team_name() {
        assert_eq!(TeamName::new("1234567").as_str(), "123456");
        assert_eq!(TeamName::new("❮✰❯").as_str(), "❮✰❯");
        assert_eq!(TeamName::new("❮✰❯").as_str(), "❮✰❯");
        assert_eq!(TeamName::new("[foo").as_str(), "foo");
        assert_eq!(TeamName::new("foo]]").as_str(), "foo");
    }
}
