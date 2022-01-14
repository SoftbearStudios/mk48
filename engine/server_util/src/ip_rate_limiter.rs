// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Helps limit the rate that a particular IP can perform an action.
pub struct IpRateLimiter {
    usage: HashMap<IpAddr, (Instant, u8)>,
    rate_limit: Duration,
    burst: u8,
    prune_counter: u8,
}

impl IpRateLimiter {
    /// Rate limit must be at least one millisecond.
    /// Burst must be less than 255.
    pub fn new(rate_limit: Duration, burst: u8) -> Self {
        debug_assert!(rate_limit.as_millis() != 0);
        debug_assert!(burst < u8::MAX);
        Self {
            usage: HashMap::new(),
            rate_limit,
            burst,
            prune_counter: 0,
        }
    }

    /// Marks the action as being performed by the ip address.
    /// Returns true if the action should be blocked (rate limited).
    pub fn limit_rate(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let (expiry, burst_used) = self.usage.entry(ip).or_insert((now, 0));

        let ok = if now > *expiry {
            *burst_used = 0;
            true
        } else if *burst_used < self.burst {
            *burst_used = burst_used.saturating_add(1);
            true
        } else {
            false
        };

        if ok {
            if let Some(instant) = expiry.checked_add(self.rate_limit) {
                *expiry = instant;
            }
        }

        self.prune_counter = self.prune_counter.wrapping_add(1);
        if self.prune_counter == 0 {
            self.prune();
        }

        !ok
    }

    /// Clean up old items. Called automatically, not it is not necesssary to call manually.
    pub fn prune(&mut self) {
        let now = Instant::now();
        self.usage
            .retain(|_, &mut (expiry, _burst_used)| expiry > now)
    }

    /// Returns size of internal data-structure.
    pub fn len(&self) -> usize {
        self.usage.len()
    }
}

#[cfg(test)]
pub mod test {
    use crate::ip_rate_limiter::IpRateLimiter;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::time::Duration;

    #[test]
    pub fn ip_rate_limiter() {
        let ip_one = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let ip_two = IpAddr::V6(Ipv6Addr::new(1, 2, 3, 4, 5, 6, 7, 8));
        let mut limiter = IpRateLimiter::new(Duration::from_millis(10), 3);

        assert_eq!(limiter.len(), 0);
        assert!(!limiter.limit_rate(ip_one));
        assert_eq!(limiter.len(), 1);
        assert!(!limiter.limit_rate(ip_one));
        assert_eq!(limiter.len(), 1);

        limiter.prune();
        assert_eq!(limiter.len(), 1);

        assert!(!limiter.limit_rate(ip_one));
        assert_eq!(limiter.len(), 1);

        limiter.prune();
        assert_eq!(limiter.len(), 1);

        assert!(limiter.limit_rate(ip_one));
        assert_eq!(limiter.len(), 1);

        std::thread::sleep(Duration::from_millis(25));

        assert!(!limiter.limit_rate(ip_two));
        assert_eq!(limiter.len(), 2);
        assert!(!limiter.limit_rate(ip_two));
        assert_eq!(limiter.len(), 2);

        limiter.prune();
        assert_eq!(limiter.len(), 2);

        std::thread::sleep(Duration::from_millis(10));

        limiter.prune();
        assert_eq!(limiter.len(), 1);

        std::thread::sleep(Duration::from_millis(50));

        limiter.prune();
        assert_eq!(limiter.len(), 0);

        assert!(!limiter.limit_rate(ip_one));
        assert_eq!(limiter.len(), 1);
    }
}
