// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

/// For rate-limiting tasks on the client (where durations are expressed in seconds).
#[derive(Debug)]
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

    /// Reset the period of a rate limiter.
    pub fn set_period(&mut self, period: f32) {
        self.period = period;
    }

    /// Takes how much time passed, in seconds, since last update.
    pub fn update(&mut self, elapsed: f32) {
        debug_assert!(elapsed >= 0.0);
        self.elapsed += elapsed;
    }

    /// Returns whether it is time to do the rate limited action, clearing the elapsed time.
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
        self.elapsed %= self.period;
        ret
    }

    /// Takes how much time passed, in seconds, since last update. Returns a iterator of possibly
    /// multiple times to do the rate-limited action. Useful if called less frequently than period.
    /// Will iterate up to a second worth of updates max.
    pub fn iter_updates(&mut self, elapsed: f32) -> impl Iterator<Item = ()> {
        self.update(elapsed);
        let iterations = (self.elapsed / self.period) as usize;
        self.elapsed -= iterations as f32 * self.period;
        std::iter::repeat(()).take(iterations.min((1.0 / self.period) as usize) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_updates() {
        let mut limiter = RateLimiter::new(0.1);
        // Starts with 1 ready (see RateLimiter::new).
        assert!(limiter.update_ready(0.0));
        assert_eq!(limiter.iter_updates(10.0).count(), 10);

        assert!(!limiter.update_ready(0.06));
        assert!(limiter.update_ready(0.06));
        assert!(!limiter.update_ready(0.06));
        assert_eq!(limiter.iter_updates(0.15).count(), 2);
    }
}
