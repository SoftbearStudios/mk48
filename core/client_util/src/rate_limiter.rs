// For rate-limiting tasks on the client (where durations are expressed in seconds).
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

    /// Takes how much time passed, in seconds, since last update. Returns whether it is time to
    /// do the rate-limited action.
    pub fn update(&mut self, elapsed: f32) -> bool {
        self.elapsed += elapsed;
        let ret = self.elapsed >= self.period;
        self.elapsed = self.elapsed % self.period;
        ret
    }
}
