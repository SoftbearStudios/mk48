// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

/// Counts frames per second over a period.
pub struct FpsMonitor {
    elapsed: f32,
    period: f32,
    frames: u32,
    last_sample: Option<f32>,
}

impl FpsMonitor {
    pub fn new(period: f32) -> Self {
        Self {
            period,
            elapsed: 0.0,
            frames: 0,
            last_sample: None,
        }
    }

    /// Returns the fps for the previous period.
    pub fn last_sample(&self) -> Option<f32> {
        self.last_sample
    }

    /// Updates the counter. Returns Some if the period has elapsed.
    pub fn update(&mut self, delta_seconds: f32) -> Option<f32> {
        self.frames = self.frames.saturating_add(1);
        self.elapsed += delta_seconds;

        if self.elapsed
            >= if self.last_sample.is_none() {
                (self.period * 0.1).max(1.0)
            } else {
                self.period
            }
        {
            let fps = self.frames as f32 / self.elapsed;
            self.elapsed = 0.0;
            self.frames = 0;
            self.last_sample = Some(fps);
            Some(fps)
        } else {
            None
        }
    }
}
