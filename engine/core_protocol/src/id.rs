// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use enum_iterator::IntoEnumIterator;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::num::{NonZeroU32, NonZeroU64, NonZeroU8};
use variant_count::VariantCount;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ArenaId(pub NonZeroU32);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum GameId {
    Mazean,
    Mk48,
    Redacted,
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

// The LanguageId enum may be extended with additional languages, such as:
// Bengali,
// Hindi,
// German,
// Japanese,
// Indonesian,
// Italy,
// Korean,
// Portuguese,
// StandardArabic,
// Vietnamese,

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LanguageId {
    #[serde(rename = "bork")]
    Bork,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "ru")]
    Russian,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "zh_TW")]
    SimplifiedChinese,
    #[serde(rename = "zh")]
    TraditionalChinese,
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
    pub fn is_bot(self) -> bool {
        let n = self.0.get();
        n & Self::DAY_MASK == 0 && !self.is_solo()
    }

    /// Returns true if the id is reserved for offline solo play.
    pub fn is_solo(self) -> bool {
        self.0.get() == 1
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RegionId {
    Asia,
    Europe,
    Usa,
}

impl Default for RegionId {
    fn default() -> Self {
        Self::Usa
    }
}

#[repr(transparent)]
/// Symbolizes, for example: #.domain.com
/// The meaning of Option::<ServerId>::None is often "localhost"
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ServerId(pub NonZeroU8);

impl ServerId {
    pub fn new(val: u8) -> Option<Self> {
        NonZeroU8::new(val).map(|nz| Self(nz))
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub NonZeroU64);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct StarId(pub NonZeroU8);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TeamId(pub NonZeroU32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
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
