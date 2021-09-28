// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use atomic_refcell::{AtomicRef, AtomicRefCell};
use lazy_static::lazy_static;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer, RingBufferExt, RingBufferWrite};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::time::{Duration, Instant};

lazy_static! {
    static ref BENCHMARKS: AtomicRefCell<BTreeMap<&'static str, Benchmark>> =
        AtomicRefCell::new(BTreeMap::new());
}

#[macro_export]
macro_rules! benchmark_scope {
    ($name:expr) => {
        let _timer = Timer::start($name);
    };
}

/// Borrows a map of all benchmarks, such as for printing out.
pub fn borrow_all() -> AtomicRef<'static, BTreeMap<&'static str, Benchmark>> {
    BENCHMARKS.borrow()
}

/// A single benchmark's timing information.
pub struct Benchmark {
    recent: ConstGenericRingBuffer<Duration, 16>,
}

impl Benchmark {
    pub fn new() -> Self {
        Self {
            recent: ConstGenericRingBuffer::new(),
        }
    }

    /// Adds one measured time to the recent history.
    fn update(&mut self, duration: Duration) {
        self.recent.push(duration);
    }
}

impl Debug for Benchmark {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // SAFETY: Benchmarks are only added when they have at least one sample, so this never
        // divides by zero.
        let mut mean = Duration::from_secs(0);
        let mut max = Duration::from_secs(0);

        for duration in self.recent.iter() {
            mean += *duration;
            max = max.max(*duration);
        }

        mean /= self.recent.len() as u32;

        write!(f, "{:.2?}/{:.2?}", mean, max)
    }
}

/// A RAII timer for benchmarks.
pub struct Timer {
    benchmark_name: &'static str,
    start: Instant,
}

impl Timer {
    #[must_use]
    pub fn start(name: &'static str) -> Self {
        Self {
            benchmark_name: name,
            start: Instant::now(),
        }
    }

    #[allow(dead_code)]
    pub fn stop(_timer: Self) {
        // Dropping the timer stops it.
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        BENCHMARKS
            .borrow_mut()
            .entry(self.benchmark_name)
            .or_insert_with(Benchmark::new)
            .update(duration);
    }
}
