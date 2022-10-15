// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::id::PlayerId;
use arrayvec::ArrayString;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::str::FromStr;
use std::sync::LazyLock;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlayerAlias(ArrayString<12>);
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Referrer(ArrayString<16>);
// TODO find a better way to limit length without copying this behemoth around on the stack.
// #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
// pub struct SurveyDetail(ArrayString<384>);
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TeamName(ArrayString<12>);

macro_rules! impl_str {
    ($typ:ty) => {
        impl $typ {
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }

            pub fn len(&self) -> usize {
                self.0.len()
            }
        }

        impl AsRef<str> for $typ {
            fn as_ref(&self) -> &str {
                self.0.as_ref()
            }
        }

        impl std::borrow::Borrow<str> for $typ {
            fn borrow(&self) -> &str {
                self.0.borrow()
            }
        }

        impl std::ops::Deref for $typ {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                &*self.0
            }
        }

        impl std::fmt::Display for $typ {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl PartialEq<str> for $typ {
            fn eq(&self, other: &str) -> bool {
                self.0.as_str() == other
            }
        }

        impl PartialOrd<str> for $typ {
            fn partial_cmp(&self, other: &str) -> Option<std::cmp::Ordering> {
                self.0.as_str().partial_cmp(other)
            }
        }
    };
}

macro_rules! impl_from_str {
    ($typ:ty) => {
        impl std::str::FromStr for $typ {
            type Err = arrayvec::CapacityError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(ArrayString::from_str(s)?))
            }
        }
    };
}

impl_str!(PlayerAlias);
impl_str!(Referrer);
// impl_str!(SurveyDetail);
impl_str!(TeamName);

impl_from_str!(PlayerAlias);
impl_from_str!(TeamName);

static BOT_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    include_str!("./famous_bots.txt")
        .split('\n')
        .filter(|s| !s.is_empty() && s.len() <= PlayerAlias::capacity())
        .collect()
});

/// A player's alias (not their real name).
impl PlayerAlias {
    /// Converts the string into a valid alias, which is never empty when done on the server.
    #[cfg(feature = "server")]
    pub fn new_sanitized(str: &str) -> Self {
        let mut string = rustrict::Censor::from_str(str)
            .with_censor_first_character_threshold(rustrict::Type::INAPPROPRIATE)
            .censor();

        let trimmed = rustrict::trim_whitespace(&string);

        if trimmed.starts_with('[') && trimmed.contains(']') {
            // Prevent alias confused with team name.
            string = string.replace('[', "<").replace(']', ">");
        }

        let ret = Self(trim_and_slice_up_to_array_string(rustrict::trim_to_width(
            &string, 14,
        )));

        return if ret.0.is_empty() {
            Self::default()
        } else {
            ret
        };
    }

    /// Doesn't trim spaces, useful for guarding text inputs.
    pub fn new_input_sanitized(str: &str) -> Self {
        Self(slice_up_to_array_string(str))
    }

    /// Good for known-good names.
    pub fn new_unsanitized(str: &str) -> Self {
        let sliced = slice_up_to_array_string(str);
        #[cfg(feature = "server")]
        debug_assert_eq!(sliced, trim_and_slice_up_to_array_string(str));
        Self(sliced)
    }

    pub fn from_bot_player_id(player_id: PlayerId) -> Self {
        // Why is this here?: debug_assert!(player_id.is_bot());
        let names = &BOT_NAMES;
        Self::new_unsanitized(names[player_id.0.get() as usize % names.len()])
    }

    fn capacity() -> usize {
        Self(ArrayString::new()).0.capacity()
    }
}

impl Default for PlayerAlias {
    fn default() -> Self {
        Self(ArrayString::from("Guest").unwrap())
    }
}

impl Referrer {
    pub const TRACKED: [&'static str; 3] = ["crazygames", "gamedistribution", "google"];

    /// For example, given `https://foo.bar.com:1234/moo.zoo/woo.hoo` the referer will be "bar".
    pub fn new(s: &str) -> Option<Self> {
        let s = s.split_once("://").map_or(s, |(_, after)| after);
        let s = s.split('/').next().unwrap();
        let mut iter = s.rsplit('.');
        iter.next().unwrap();
        let s = if let Some(second_from_last) = iter.next() {
            // e.g. "foo.com.uk"
            matches!(second_from_last, "co" | "com")
                .then(|| iter.next())
                .flatten()
                .unwrap_or(second_from_last)
        } else {
            // e.g. localhost
            s
        };
        (!s.is_empty()).then(|| Self(slice_up_to_array_string(s)))
    }
}

impl FromStr for Referrer {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[cfg(target = "server")]
        return Ok(Self(trim_and_slice_up_to_array_string(s)));
        #[cfg(not(target = "server"))]
        Ok(Self(slice_up_to_array_string(s)))
    }
}

impl TeamName {
    const MAX_CHARS: usize = 6;
    /// In units of `m`.
    #[cfg(feature = "server")]
    const MAX_WIDTH: usize = 8;

    pub fn new_unsanitized(str: &str) -> Self {
        let sliced = slice_up_to_array_string(str);
        #[cfg(feature = "server")]
        debug_assert_eq!(sliced, trim_and_slice_up_to_array_string(str));
        Self(sliced)
    }

    /// Enforces `MAX_CHARS`, doesn't trim spaces, useful for guarding text inputs.
    pub fn new_input_sanitized(str: &str) -> Self {
        Self(slice_up_to_array_string(slice_up_to_chars(
            str,
            Self::MAX_CHARS,
        )))
    }

    #[cfg(feature = "server")]
    pub fn new_sanitized(str: &str) -> Self {
        let string = rustrict::Censor::from_str(str)
            .with_censor_first_character_threshold(rustrict::Type::INAPPROPRIATE)
            .take(Self::MAX_CHARS)
            .collect::<String>();

        let str = rustrict::trim_whitespace(rustrict::trim_to_width(&string, Self::MAX_WIDTH))
            .trim_start_matches('[')
            .trim_end_matches(']');

        Self::new_unsanitized(str)
    }
}

#[cfg(feature = "server")]
pub fn trim_and_slice_up_to(s: &str, bytes: usize) -> &str {
    slice_up_to_bytes(rustrict::trim_whitespace(s), bytes)
}

fn slice_up_to_bytes(s: &str, bytes: usize) -> &str {
    let mut idx = bytes;
    while !s.is_char_boundary(idx) {
        idx -= 1;
    }
    &s[..idx]
}

fn slice_up_to_chars(s: &str, max: usize) -> &str {
    &s[0..s
        .char_indices()
        .nth(max)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())]
}

#[cfg(feature = "server")]
pub fn trim_and_slice_up_to_array_string<const CAPACITY: usize>(s: &str) -> ArrayString<CAPACITY> {
    ArrayString::from(trim_and_slice_up_to(s, CAPACITY)).unwrap()
}

pub fn slice_up_to_array_string<const CAPACITY: usize>(s: &str) -> ArrayString<CAPACITY> {
    ArrayString::from(slice_up_to_bytes(s, CAPACITY)).unwrap()
}

#[cfg(test)]
mod test {
    use crate::name::Referrer;

    #[test]
    #[cfg(feature = "server")]
    fn team_name() {
        use crate::name::TeamName;

        assert_eq!(TeamName::new_sanitized("1234567").as_str(), "123456");
        assert_eq!(TeamName::new_sanitized("❮✰❯").as_str(), "❮✰❯");
        assert_eq!(TeamName::new_sanitized("❮✰❯").as_str(), "❮✰❯");
        assert_eq!(TeamName::new_sanitized("[foo").as_str(), "foo");
        assert_eq!(TeamName::new_sanitized("foo]]").as_str(), "foo");
    }

    #[test]
    fn referrer() {
        assert_eq!(&Referrer::new("http://foo.bar.com").unwrap(), "bar");
        assert_eq!(&Referrer::new("baz.xyz").unwrap(), "baz");
        assert_eq!(&Referrer::new("foo.com.uk").unwrap(), "foo");
        assert_eq!(&Referrer::new("com.uk").unwrap(), "com");
        assert_eq!(
            &Referrer::new("https://one.two.three.four/five.html").unwrap(),
            "three"
        );
        assert_eq!(Referrer::new(""), None);
    }
}
