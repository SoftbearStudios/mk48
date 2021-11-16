// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

/// Monitors updates per second
pub struct UpsMonitor {
    start: Instant,
    updates_since_start: u16,
    previous: f32,
}

impl UpsMonitor {
    const UPDATES_PER_SAMPLE: u16 = 1000;

    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            updates_since_start: 0,
            previous: f32::NAN,
        }
    }

    pub fn update(&mut self) -> Option<f32> {
        if self.updates_since_start >= Self::UPDATES_PER_SAMPLE {
            let ret = (self.updates_since_start as f32) / self.start.elapsed().as_secs_f32();
            self.start = Instant::now();
            self.updates_since_start = 0;
            if (ret - self.previous).abs() < 0.001 {
                None
            } else {
                self.previous = ret;
                Some(ret)
            }
        } else {
            self.updates_since_start += 1;
            None
        }
    }
}
