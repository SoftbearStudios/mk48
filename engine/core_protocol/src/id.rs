// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use enum_iterator::IntoEnumIterator;
use lazy_static::lazy_static;
use rand::distributions::{Standard, WeightedIndex};
use rand::prelude::Distribution;
use rand::Rng;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Display, Formatter};
use std::num::{NonZeroU32, NonZeroU64, NonZeroU8, ParseIntError};
use std::str::FromStr;
use variant_count::VariantCount;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ArenaId(pub NonZeroU32);

/// Cohorts 1-4 are used for A/B testing.
/// The default for existing players is cohort 1.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CohortId(pub NonZeroU8);

impl CohortId {
    const WEIGHTS: [u8; 4] = [8, 4, 2, 1];

    pub fn new(n: u8) -> Option<Self> {
        NonZeroU8::new(n)
            .filter(|n| n.get() <= Self::WEIGHTS.len() as u8)
            .map(Self)
    }
}

impl Default for CohortId {
    fn default() -> Self {
        Self::new(1).unwrap()
    }
}

impl Display for CohortId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
pub struct InvalidCohortId;

impl FromStr for CohortId {
    type Err = InvalidCohortId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().ok().and_then(Self::new).ok_or(InvalidCohortId)
    }
}

impl Distribution<CohortId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CohortId {
        lazy_static! {
            static ref DISTRIBUTION: WeightedIndex<u8> =
                WeightedIndex::new(&CohortId::WEIGHTS).unwrap();
        }
        let n = DISTRIBUTION.sample(rng) + 1;
        debug_assert!(n > 0);
        debug_assert!(n <= CohortId::WEIGHTS.len());
        // The or default is purely defensive.
        CohortId::new(n as u8).unwrap_or_default()
    }
}

impl Serialize for CohortId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.get().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CohortId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        <u8>::deserialize(deserializer)
            .and_then(|n| Self::new(n).ok_or(D::Error::custom("invalid cohort id")))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum GameId {
    Mk48,
    /// A placeholder for games we haven't released yet.
    Redacted,
}

impl GameId {
    pub fn name(self) -> &'static str {
        match self {
            Self::Mk48 => "Mk48.io",
            Self::Redacted => "Redacted",
        }
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct InvitationId(pub NonZeroU32);

impl InvitationId {
    pub fn generate(server_id: Option<ServerId>) -> Self {
        let mut r: u32 = rand::thread_rng().gen();
        if r == 0 {
            r = 1;
        }
        Self(
            NonZeroU32::new(
                ((server_id.map(|id| id.0.get()).unwrap_or(0) as u32) << 24)
                    | (r & ((1 << 24) - 1)),
            )
            .unwrap(),
        )
    }

    pub fn server_id(self) -> Option<ServerId> {
        NonZeroU8::new((self.0.get() >> 24) as u8).map(|nz| ServerId(nz))
    }
}

impl Display for InvitationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl FromStr for InvitationId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NonZeroU32::from_str(s).map(InvitationId)
    }
}

// The LanguageId enum may be extended with additional languages, such as:
// Bengali,
// Hindi,
// Indonesian,
// Italy,
// Korean,
// Portuguese,
// StandardArabic,
// TraditionalChinese,

/// In order that they should be presented in a language picker.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, IntoEnumIterator)]
pub enum LanguageId {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "ru")]
    Russian,
    #[serde(rename = "ar")]
    Arabic,
    #[serde(rename = "zh")]
    SimplifiedChinese,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "vi")]
    Vietnamese,
    #[serde(rename = "xx-bork")]
    Bork,
}

impl LanguageId {
    pub fn iter() -> impl Iterator<Item = Self> + 'static {
        Self::into_enum_iter()
    }
}

impl Default for LanguageId {
    fn default() -> Self {
        Self::English
    }
}

/// `PeriodId` is used by `LeaderboardDto`.
#[derive(
    Clone, Copy, Debug, Hash, Eq, PartialEq, Deserialize, IntoEnumIterator, Serialize, VariantCount,
)]
pub enum PeriodId {
    AllTime = 0,
    Daily = 1,
    Weekly = 2,
}

impl From<usize> for PeriodId {
    fn from(i: usize) -> Self {
        match i {
            0 => Self::AllTime,
            1 => Self::Daily,
            2 => Self::Weekly,
            _ => panic!("invalid index"),
        }
    }
}

impl PeriodId {
    pub fn iter() -> impl Iterator<Item = Self> {
        Self::into_enum_iter()
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub NonZeroU32);

impl PlayerId {
    pub const DAY_BITS: u32 = 10;
    pub const RANDOM_BITS: u32 = 32 - Self::DAY_BITS;
    pub const RANDOM_MASK: u32 = (1 << Self::RANDOM_BITS) - 1;
    pub const DAY_MASK: u32 = !Self::RANDOM_MASK;

    /// The player ID of the solo player in offline single player mode.
    /// TODO: This is not currently used.
    pub const SOLO_OFFLINE: Self = Self(NonZeroU32::new(1).unwrap());

    /// Gets the bot number associated with this id, or [`None`] if the id is not a bot.
    pub fn bot_number(self) -> Option<usize> {
        self.is_bot().then_some(self.0.get() as usize - 2)
    }

    /// Gets the nth id associated with bots.
    pub fn nth_bot(n: usize) -> Option<Self> {
        debug_assert!(n <= u32::MAX as usize - 1);
        NonZeroU32::new(n as u32 + 2)
            .map(Self)
            .filter(|id| id.is_bot())
    }

    /// Returns true if the id is reserved for bots.
    pub const fn is_bot(self) -> bool {
        let n = self.0.get();
        n & Self::DAY_MASK == 0 && !self.is_solo()
    }

    /// Returns true if the id is reserved for offline solo play.
    pub const fn is_solo(self) -> bool {
        self.0.get() == 1
    }
}

/// Mirrors [`db_ip::Region`]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, IntoEnumIterator, Serialize)]
pub enum RegionId {
    Africa,
    Asia,
    Europe,
    NorthAmerica,
    Oceania,
    SouthAmerica,
}

impl Default for RegionId {
    fn default() -> Self {
        Self::NorthAmerica
    }
}

impl RegionId {
    /// Returns a relative distance to another region.
    /// It is not necessarily transitive.
    pub fn distance(self, other: Self) -> u8 {
        match self {
            Self::Africa => match other {
                Self::Africa => 0,
                Self::Asia => 2,
                Self::Europe => 1,
                Self::NorthAmerica => 2,
                Self::Oceania => 3,
                Self::SouthAmerica => 3,
            },
            Self::Asia => match other {
                Self::Africa => 2,
                Self::Asia => 0,
                Self::Europe => 2,
                Self::NorthAmerica => 2,
                Self::Oceania => 1,
                Self::SouthAmerica => 3,
            },
            Self::Europe => match other {
                Self::Africa => 1,
                Self::Asia => 2,
                Self::Europe => 0,
                Self::NorthAmerica => 2,
                Self::Oceania => 3,
                Self::SouthAmerica => 3,
            },
            Self::NorthAmerica => match other {
                Self::Africa => 3,
                Self::Asia => 3,
                Self::Europe => 2,
                Self::NorthAmerica => 0,
                Self::Oceania => 2,
                Self::SouthAmerica => 1,
            },
            Self::Oceania => match other {
                Self::Africa => 3,
                Self::Asia => 1,
                Self::Europe => 2,
                Self::NorthAmerica => 2,
                Self::Oceania => 0,
                Self::SouthAmerica => 3,
            },
            Self::SouthAmerica => match other {
                Self::Africa => 3,
                Self::Asia => 2,
                Self::Europe => 2,
                Self::NorthAmerica => 1,
                Self::Oceania => 2,
                Self::SouthAmerica => 0,
            },
        }
    }

    pub fn as_human_readable_str(self) -> &'static str {
        match self {
            Self::Africa => "Africa",
            Self::Asia => "Asia",
            Self::Europe => "Europe",
            Self::NorthAmerica => "North America",
            Self::Oceania => "Oceania",
            Self::SouthAmerica => "South America",
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> + 'static {
        Self::into_enum_iter()
    }
}

/// Wasn't a valid region string.
#[derive(Debug)]
pub struct InvalidRegionId;

impl Display for InvalidRegionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "invalid region id string")
    }
}

impl FromStr for RegionId {
    type Err = InvalidRegionId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "af" | "africa" => Self::Africa,
            "as" | "asia" => Self::Asia,
            "eu" | "europe" => Self::Europe,
            "na" | "northamerica" => Self::NorthAmerica,
            "oc" | "oceania" => Self::Oceania,
            "sa" | "southamerica" => Self::SouthAmerica,
            _ => return Err(InvalidRegionId),
        })
    }
}

#[repr(transparent)]
/// Symbolizes, for example: #.domain.com
/// The meaning of Option::<ServerId>::None is often "localhost"
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ServerId(pub NonZeroU8);

impl ServerId {
    pub fn new(val: u8) -> Option<Self> {
        NonZeroU8::new(val).map(Self)
    }
}

impl Display for ServerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub NonZeroU64);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TeamId(pub NonZeroU32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, IntoEnumIterator, Serialize, Deserialize)]
pub enum UserAgentId {
    ChromeOS,
    Desktop,
    DesktopChrome,
    DesktopFirefox,
    DesktopSafari,
    Mobile,
    Spider,
    Tablet,
}

impl UserAgentId {
    pub fn iter() -> impl Iterator<Item = Self> + 'static {
        Self::into_enum_iter()
    }
}

// This will supersede [`PlayerId`] for persistent storage.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct UserId(pub NonZeroU64);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum LoginType {
    /// Discord OAuth2.
    Discord,
}

#[cfg(test)]
mod tests {
    use crate::id::{InvitationId, PlayerId, ServerId};
    use std::num::NonZeroU8;

    #[test]
    fn invitation_id() {
        for i in 1..=u8::MAX {
            let sid = ServerId(NonZeroU8::new(i).unwrap());
            let iid = InvitationId::generate(Some(sid));
            assert_eq!(iid.server_id(), Some(sid));
        }
    }

    #[test]
    fn solo() {
        assert!(PlayerId::SOLO_OFFLINE.is_solo());
    }
}
