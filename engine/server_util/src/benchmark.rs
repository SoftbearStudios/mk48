// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use atomic_refcell::{AtomicRef, AtomicRefCell};
use core_protocol::metrics::ContinuousExtremaMetric;
use lazy_static::lazy_static;
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

pub use benchmark_scope;

/// Borrows a map of all benchmarks, such as for printing out.
pub fn borrow_all() -> AtomicRef<'static, BTreeMap<&'static str, Benchmark>> {
    BENCHMARKS.borrow()
}

pub fn reset_all() {
    BENCHMARKS
        .borrow_mut()
        .values_mut()
        .for_each(Benchmark::reset);
}

/// A single benchmark's timing information.
#[derive(Default)]
pub struct Benchmark {
    timings: ContinuousExtremaMetric,
}

impl Benchmark {
    /// Adds one measured time to the recent history.
    fn update(&mut self, duration: Duration) {
        self.timings.push(duration.as_secs_f32());
    }

    /// Resets timing information.
    fn reset(&mut self) {
        self.timings = ContinuousExtremaMetric::default();
    }
}

impl Debug for Benchmark {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3?}/{:.3?}", self.timings.average(), self.timings.max)
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
            .or_insert_with(Benchmark::default)
            .update(duration);
    }
}
