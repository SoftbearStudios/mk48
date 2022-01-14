// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

// Angle is very sharable code. However, it is more game related than core_protocol related, so
// it is imported from common.
pub use common_util::ticks::{Ticks, TicksRepr};

/// REGEN_DAMAGE controls how long it takes to regenerate one unit of damage.
const REGEN_DAMAGE: Ticks = Ticks((Ticks::FREQUENCY_HZ.0 * 60) as TicksRepr);

/// from_damage returns the amount of Ticks required to regenerate a given amount of damage.
/// TODO: Eliminate the concept of damage entirely, and only use Ticks.
pub fn from_damage(damage: f32) -> Ticks {
    Ticks::from_secs(damage * REGEN_DAMAGE.to_secs())
}
