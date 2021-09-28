// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use arrayvec::ArrayString;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

/// An alias, e.g. "mrbig", is NOT a real name.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlayerAlias(pub ArrayString<12>);
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Referer(pub ArrayString<16>);
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ServerAddr(pub ArrayString<32>);
#[repr(transparent)]
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

impl ServerAddr {
    pub fn new(str: &str) -> Self {
        Self(slice_up_to(str))
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
    // NOTE: The following characters are not detected by
    // is_whitespace() but show up as blank.

    // https://www.compart.com/en/unicode/U+2800
    // https://www.compart.com/en/unicode/U+200B
    // https://www.compart.com/en/unicode/U+3164
    s.trim_matches(|c: char| {
        c.is_whitespace() || c == '\u{2800}' || c == '\u{200B}' || c == '\u{3164}'
    })
}

pub type Location = Vec3;
