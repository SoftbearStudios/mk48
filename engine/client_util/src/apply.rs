// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

/// Resettable data build from updates.
pub trait Apply<U>: Default {
    /// Applies an inbound update to the state.
    fn apply(&mut self, update: U);
    /// Resets the state to default.
    fn reset(&mut self) {
        *self = Self::default();
    }
}

impl Apply<()> for () {
    fn apply(&mut self, _update: ()) {}
}
