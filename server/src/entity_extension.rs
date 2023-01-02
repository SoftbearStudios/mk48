// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use common::altitude::Altitude;
use common::angle::Angle;
use common::entity::*;
use common::ticks::Ticks;
use common::util::make_mut_slice;
use common_util::alloc::{arc_default_n, box_default_n};
use std::iter::FromIterator;
use std::sync::Arc;

/// Additional fields for certain entities (for now, boats). Stored separately for memory efficiency.
#[derive(Debug)]
pub struct EntityExtension {
    // true means altitude target is Altitude::MIN false means Altitude::ZERO.
    // Used by Self::altitude_target().
    // Can't submerge right away to prevent dodging missiles.
    submerge: bool,
    submerge_delay: Ticks,

    /// Whether the player *wants* active sensors. To tell if the player *has* active sensors, use
    /// Used by Self::is_active().
    /// Active stays on for a an extra duration to avoid rapid switching, which could induce flickering on other player's screens.
    active: bool,
    deactivate_delay: Ticks,

    /// Ticks of protection ticks remaining, zeroed if showing signs of aggression.
    spawn_protection_remaining: Ticks,

    // 1 reload per armament, 0 = reloaded.
    // Not an arc because converted to a bitset with max len of 32.
    pub reloads: Box<[Ticks]>,

    // 1 angle per turret relative to boat.
    // Arc to save allocations
    pub turrets: Arc<[Angle]>,
}

impl EntityExtension {
    /// How long spawn protection lasts (it linearly fades over this time).
    const SPAWN_PROTECTION_INITIAL: Ticks = Ticks::from_whole_secs(20);

    /// How long deactivating sensors is delayed.
    const DEACTIVATE_DELAY: Ticks = Ticks::from_repr(5);
    /// How long submerging is delayed.
    const SUBMERGE_DELAY: Ticks = Ticks::from_repr(8);

    /// Allocates reloads and turrets, sized to a particular entity type.
    /// It can also give spawn protection.
    pub fn change_entity_type(&mut self, entity_type: EntityType) {
        // TODO clear active/submerge based on if boat supports them but probably doesn't matter.

        let data = entity_type.data();
        self.spawn_protection_remaining = if entity_type.data().level == 1 {
            Self::SPAWN_PROTECTION_INITIAL
        } else {
            Ticks::ZERO
        };
        self.reloads = box_default_n(data.armaments.len());
        self.turrets = Arc::from_iter(data.turrets.iter().map(|t| t.angle));
    }

    /// Returns the target altitude of the boat from submerge.
    pub fn altitude_target(&self) -> Altitude {
        if self.submerge && self.submerge_delay == Ticks::ZERO {
            Altitude::MIN
        } else {
            Altitude::ZERO
        }
    }

    /// Sets submerge, possibly also setting deactivate_delay to an appropriate value.
    pub fn set_submerge(&mut self, submerge: bool) {
        if submerge && !self.submerge {
            self.submerge_delay = Self::SUBMERGE_DELAY;
        }
        self.submerge = submerge;
    }

    /// Returns whether active sensors, or within deactivate sensor delay.
    pub fn is_active(&self) -> bool {
        self.active || self.deactivate_delay > Ticks::ZERO
    }

    /// Sets active, possibly also setting deactivate_delay to an appropriate value.
    pub fn set_active(&mut self, active: bool) {
        if !active && self.active {
            self.deactivate_delay = Self::DEACTIVATE_DELAY;
        }
        self.active = active;
    }

    /// Returns a multiplier for damage taken, taking into account spawn protection.
    pub fn spawn_protection(&self) -> f32 {
        (Self::SPAWN_PROTECTION_INITIAL - self.spawn_protection_remaining).to_secs()
            / Self::SPAWN_PROTECTION_INITIAL.to_secs()
    }

    /// Clears any remaining spawn protection (useful if showing signs of aggression, and thus
    /// no longer deserving of spawn protection).
    pub fn clear_spawn_protection(&mut self) {
        self.spawn_protection_remaining = Ticks::ZERO;
    }

    /// Subtracts from the player's tickers:
    /// submerge
    /// deactivate_delay
    /// spawn_protection_remaining
    pub fn update_tickers(&mut self, delta: Ticks) {
        self.submerge_delay = self.submerge_delay.saturating_sub(delta);
        self.deactivate_delay = self.deactivate_delay.saturating_sub(delta);
        self.spawn_protection_remaining = self.spawn_protection_remaining.saturating_sub(delta);
    }

    /// reloads_mut returns a mutable reference to the reloads component of the extension.
    pub fn reloads_mut(&mut self) -> &mut [Ticks] {
        &mut self.reloads
    }

    /// reloads_mut returns a mutable reference to the turret angles component of the extension.
    pub fn turrets_mut(&mut self) -> &mut [Angle] {
        make_mut_slice(&mut self.turrets)
    }
}

impl Default for EntityExtension {
    /// default allocates an empty entity extension, suitable as not having a boat.
    /// Once a boat is spawned/upgraded change_entity_type must be called.
    fn default() -> Self {
        Self {
            submerge: false,
            submerge_delay: Ticks::ZERO,
            active: true,
            deactivate_delay: Ticks::ZERO,
            spawn_protection_remaining: Self::SPAWN_PROTECTION_INITIAL,
            reloads: box_default_n(0),
            turrets: arc_default_n(0),
        }
    }
}
