// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

/// A rate limiter that may have unique properties.
pub struct RateLimiter {
    props: RateLimiterProps,
    state: RateLimiterState,
}

/// The state of a rate limiter.
pub struct RateLimiterState {
    pub(crate) until: Instant,
    pub(crate) burst_used: u8,
}

/// The (sharable) properties of a rate limiter.
pub struct RateLimiterProps {
    rate_limit: Duration,
    burst: u8,
}

impl RateLimiterState {
    /// Returns true if the action exceeds the rate limit defined by the props and should be prevented.
    pub fn should_limit_rate(&mut self, props: &RateLimiterProps) -> bool {
        self.should_limit_rate_with_now(props, Instant::now())
    }

    /// Like [`Self::should_rate_limit`] but more efficient if you already know the current time.
    pub fn should_limit_rate_with_now(&mut self, props: &RateLimiterProps, now: Instant) -> bool {
        if props.rate_limit == Duration::ZERO {
            return false;
        }

        let ok = if now > self.until {
            self.burst_used = 0;
            true
        } else if self.burst_used < props.burst {
            self.burst_used = self.burst_used.saturating_add(1);
            true
        } else {
            false
        };

        if ok {
            if let Some(instant) = self.until.checked_add(props.rate_limit) {
                self.until = instant;
            }
        }

        !ok
    }
}

impl Default for RateLimiterState {
    fn default() -> Self {
        Self {
            until: Instant::now(),
            burst_used: 0,
        }
    }
}

impl RateLimiterProps {
    /// rate limit should be more than zero.
    /// burst must be less than [`u8::MAX`].
    pub fn new(rate_limit: Duration, burst: u8) -> Self {
        debug_assert!(
            rate_limit.as_millis() != 0,
            "use RateLimiterProps::no_limit() to explicitly opt out of rate limiting"
        );
        debug_assert!(burst < u8::MAX);
        Self { rate_limit, burst }
    }

    /// Properties of a rate limiter that allow infinite rate.
    pub fn no_limit() -> Self {
        Self {
            rate_limit: Duration::ZERO,
            burst: 0,
        }
    }
}

impl RateLimiter {
    /// Creates a new rate limiter with the specified properties.
    pub fn new(rate_limit: Duration, burst: u8) -> Self {
        Self::from(RateLimiterProps::new(rate_limit, burst))
    }

    /// Constructs a rate limiter that allows infinite rate.
    pub fn no_limit() -> Self {
        Self::from(RateLimiterProps::no_limit())
    }

    /// Returns true if the action exceeds the rate limit and should be prevented.
    pub fn should_limit_rate(&mut self) -> bool {
        self.state.should_limit_rate(&self.props)
    }

    /// Like [`Self::should_limit_rate`] but more efficient if you already know the time.
    pub fn should_limit_rate_with_now(&mut self, now: Instant) -> bool {
        self.state.should_limit_rate_with_now(&self.props, now)
    }
}

impl From<RateLimiterProps> for RateLimiter {
    fn from(props: RateLimiterProps) -> Self {
        Self {
            props,
            state: RateLimiterState::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::rate_limiter::RateLimiter;
    use std::time::Duration;

    #[test]
    fn normal() {
        let mut rate_limiter = RateLimiter::new(Duration::from_millis(10), 2);

        for _ in 0..10 {
            assert!(!rate_limiter.should_limit_rate());
            assert_eq!(rate_limiter.state.burst_used, 0);
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    #[test]
    fn limit_exceeded() {
        let mut rate_limiter = RateLimiter::new(Duration::from_millis(10), 3);

        std::thread::sleep(Duration::from_millis(5));

        assert!(!rate_limiter.should_limit_rate());
        assert_eq!(rate_limiter.state.burst_used, 0);
        assert!(!rate_limiter.should_limit_rate());
        assert_eq!(rate_limiter.state.burst_used, 1);
        assert!(!rate_limiter.should_limit_rate());
        assert_eq!(rate_limiter.state.burst_used, 2);
        assert!(!rate_limiter.should_limit_rate());
        assert_eq!(rate_limiter.state.burst_used, 3);
        assert!(rate_limiter.should_limit_rate());
        assert_eq!(rate_limiter.state.burst_used, 3);

        std::thread::sleep(Duration::from_millis(50));

        assert!(!rate_limiter.should_limit_rate());
        assert_eq!(rate_limiter.state.burst_used, 0);
    }

    #[test]
    fn no_limit() {
        let mut rate_limiter = RateLimiter::no_limit();

        for _ in 0..1000 {
            assert!(!rate_limiter.should_limit_rate());
        }
    }
}
