// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

/// For rate-limiting tasks on the client (where durations are expressed in seconds).
pub struct RateLimiter {
    elapsed: f32,
    period: f32,
}

impl RateLimiter {
    pub fn new(period: f32) -> Self {
        Self {
            elapsed: period,
            period,
        }
    }

    /// Fast tracks the next update to return true.
    pub fn fast_track(&mut self) {
        self.elapsed = self.period;
    }

    /// Takes how much time passed, in seconds, since last update.
    pub fn update(&mut self, elapsed: f32) {
        debug_assert!(elapsed >= 0.0);
        self.elapsed += elapsed;
    }

    /// Returns whether it is time to do the rate limited action, cleari
    pub fn ready(&mut self) -> bool {
        let ret = self.elapsed >= self.period;
        self.elapsed = 0.0;
        ret
    }

    /// Takes how much time passed, in seconds, since last update. Returns whether it is time to
    /// do the rate-limited action.
    pub fn update_ready(&mut self, elapsed: f32) -> bool {
        self.update(elapsed);
        let ret = self.elapsed >= self.period;
        self.elapsed = self.elapsed % self.period;
        ret
    }
}
