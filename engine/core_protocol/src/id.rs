// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Display, Formatter};
use std::num::{NonZeroU32, NonZeroU64, NonZeroU8};
use std::str::FromStr;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

#[cfg(feature = "server")]
use rand::distributions::{Standard, WeightedIndex};
#[cfg(feature = "server")]
use rand::prelude::*;

macro_rules! impl_wrapper_from_str {
    ($typ:ty, $inner:ty) => {
        impl std::fmt::Display for $typ {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $typ {
            type Err = <$inner as FromStr>::Err;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(FromStr::from_str(s)?))
            }
        }
    };
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ArenaId(pub NonZeroU32);
impl_wrapper_from_str!(ArenaId, NonZeroU32);

/// Cohorts 1-4 are used for A/B testing.
/// The default for existing players is cohort 1.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CohortId(pub NonZeroU8);
impl_wrapper_from_str!(CohortId, NonZeroU8);

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

#[cfg(feature = "server")]
impl Distribution<CohortId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CohortId {
        use std::sync::LazyLock;
        static DISTRIBUTION: LazyLock<WeightedIndex<u8>> =
            LazyLock::new(|| WeightedIndex::new(&CohortId::WEIGHTS).unwrap());

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
    Kiomet,
    Mk48,
    /// A placeholder for games we haven't released yet.
    Redacted,
}

impl GameId {
    pub fn name(self) -> &'static str {
        match self {
            Self::Kiomet => "Kiomet",
            Self::Mk48 => "Mk48.io",
            Self::Redacted => "Redacted",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct InvitationId(pub NonZeroU32);

impl InvitationId {
    #[cfg(feature = "server")]
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

impl_wrapper_from_str!(InvitationId, NonZeroU32);

// The LanguageId enum may be extended with additional languages, such as:
// Bengali,
// Hindi,
// Indonesian,
// Korean,
// Portuguese,
// TraditionalChinese,

/// In order that they should be presented in a language picker.
#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumIter, EnumString, Display)]
pub enum LanguageId {
    #[strum(serialize = "en")]
    English,
    #[strum(serialize = "es")]
    Spanish,
    #[strum(serialize = "fr")]
    French,
    #[strum(serialize = "de")]
    German,
    #[strum(serialize = "it")]
    Italian,
    #[strum(serialize = "ru")]
    Russian,
    #[strum(serialize = "ar")]
    Arabic,
    #[strum(serialize = "hi")]
    Hindi,
    #[strum(serialize = "zh")]
    SimplifiedChinese,
    #[strum(serialize = "ja")]
    Japanese,
    #[strum(serialize = "vi")]
    Vietnamese,
    #[strum(serialize = "xx-bork")]
    Bork,
}

impl LanguageId {
    pub fn iter() -> impl Iterator<Item = Self> + 'static {
        <Self as IntoEnumIterator>::iter()
    }
}

impl Default for LanguageId {
    fn default() -> Self {
        Self::English
    }
}

/// `PeriodId` is used by `LeaderboardDto`.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Deserialize, EnumIter, Serialize)]
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
        <Self as IntoEnumIterator>::iter()
    }
}

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

/// Mirrors <https://github.com/finnbear/db_ip>: `Region`.
/// TODO use strum to implement FromStr
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, EnumIter, Serialize)]
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
        <Self as IntoEnumIterator>::iter()
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
        Ok(match s.to_ascii_lowercase().as_str() {
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

/// Symbolizes, for example: #.domain.com
/// The meaning of [`Option::<ServerId>::None`] is often "localhost"
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ServerId(pub NonZeroU8);
impl_wrapper_from_str!(ServerId, NonZeroU8);

impl ServerId {
    pub fn new(val: u8) -> Option<Self> {
        NonZeroU8::new(val).map(Self)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub NonZeroU64);
impl_wrapper_from_str!(SessionId, NonZeroU64);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TeamId(pub NonZeroU32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, EnumIter, Serialize, Deserialize)]
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
        <Self as IntoEnumIterator>::iter()
    }
}

// This will supersede [`PlayerId`] for persistent storage.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct UserId(pub NonZeroU64);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum LoginType {
    /// Discord OAuth2.
    Discord,
}

#[cfg(test)]
mod tests {
    use crate::id::PlayerId;

    #[test]
    #[cfg(feature = "server")]
    fn invitation_id() {
        use crate::id::{InvitationId, ServerId};
        use std::num::NonZeroU8;

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
