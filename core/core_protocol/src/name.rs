// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use arrayvec::ArrayString;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};
use unicode_categories::UnicodeCategories;

/// An alias, e.g. "mrbig", is NOT a real name.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlayerAlias(pub ArrayString<12>);
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Referrer(pub ArrayString<16>);
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SurveyDetail(pub ArrayString<384>);
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TeamName(pub ArrayString<12>);

impl PlayerAlias {
    pub fn new(str: &str) -> Self {
        Self(slice_up_to(str))
    }

    pub fn capacity() -> usize {
        Self(ArrayString::new()).0.capacity()
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
            let cooked = e[n - 2];
            Some(Self(slice_up_to(cooked)))
        } else {
            None
        }
    }
}

impl TeamName {
    pub fn new(str: &str) -> Self {
        Self(slice_up_to(str))
    }
}

impl Display for TeamName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub fn slice_up_to<const CAPACITY: usize>(s: &str) -> ArrayString<CAPACITY> {
    let s = trim_spaces(s);
    let mut idx = CAPACITY;
    while !s.is_char_boundary(idx) {
        idx -= 1;
    }
    ArrayString::from(&s[..idx]).unwrap()
}

pub fn trim_spaces(s: &str) -> &str {
    // NOTE: The following characters are not detected by standard means but show up as blank.
    // https://www.compart.com/en/unicode/U+2800
    // https://www.compart.com/en/unicode/U+3164
    s.trim_matches(|c: char| {
        c.is_whitespace() || c.is_other() || c == '\u{2800}' || c == '\u{3164}'
    })
}

pub type Location = Vec3;
