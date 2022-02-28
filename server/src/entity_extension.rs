// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use common::altitude::Altitude;
use common::angle::Angle;
use common::entity::*;
use common::ticks::Ticks;
use common::util::make_mut_slice;
use std::iter::FromIterator;
use std::sync::Arc;

/// Additional fields for certain entities (for now, boats). Stored separately for memory efficiency.
#[derive(Debug)]
pub struct EntityExtension {
    pub altitude_target: Altitude,
    /// Whether the player *wants* active sensors. To tell if the player *has* active sensors, use
    /// Self::is_active() instead.
    pub active: bool,
    /// Active stays on for a an extra duration to avoid rapid switching, which could induce flickering on other player's screens.
    active_cooldown: Ticks,
    /// Ticks of protection ticks remaining, zeroed if showing signs of aggression.
    spawn_protection_remaining: Ticks,
    pub reloads: Box<[Ticks]>,
    pub turrets: Arc<[Angle]>,
}

fn arc_default_n<T: Default>(n: usize) -> Arc<[T]> {
    Arc::from_iter((0..n).map(|_| T::default()))
}

fn box_default_n<T: Default>(n: usize) -> Box<[T]> {
    Box::from_iter((0..n).map(|_| T::default()))
}

impl EntityExtension {
    /// How long spawn protection lasts (it linearly fades over this time).
    const SPAWN_PROTECTION_INITIAL: Ticks = Ticks(Ticks::FREQUENCY_HZ.0 * 20);

    /// new allocates a new entity extension, sized to a particular entity type.
    pub fn new(entity_type: EntityType) -> Self {
        let data = entity_type.data();
        Self {
            altitude_target: Altitude::ZERO,
            active: true,
            active_cooldown: Ticks::ZERO,
            spawn_protection_remaining: if entity_type.data().level == 1 {
                Self::SPAWN_PROTECTION_INITIAL
            } else {
                Ticks::ZERO
            },
            reloads: box_default_n(data.armaments.len()),
            turrets: Arc::from_iter(data.turrets.iter().map(|t| t.angle)),
        }
    }

    /// Returns whether active sensors, or within active sensor cooldown.
    pub fn is_active(&self) -> bool {
        self.active || self.active_cooldown > Ticks::ZERO
    }

    /// Sets active, possibly also setting active_cooldown to an appropriate value.
    pub fn set_active(&mut self, active: bool) {
        if !active && self.active {
            self.active_cooldown = Ticks::from_secs(0.5);
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

    /// Subtracts from the active cooldown and spawn protection until they reach zero.
    pub fn update_active_cooldown_and_spawn_protection(&mut self, delta: Ticks) {
        self.active_cooldown = self.active_cooldown.saturating_sub(delta);
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
    /// default allocates an empty entity extension, suitable only as a placeholder.
    fn default() -> Self {
        Self {
            altitude_target: Altitude::ZERO,
            active: true,
            spawn_protection_remaining: Self::SPAWN_PROTECTION_INITIAL,
            active_cooldown: Ticks::ZERO,
            reloads: box_default_n(0),
            turrets: arc_default_n(0),
        }
    }
}
