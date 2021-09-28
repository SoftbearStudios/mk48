// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::ops::Add;

pub trait Metric: Sized + Add {
    type Summary;

    fn summarize(&self) -> Self::Summary;
}

/// A metric representing something countable.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct DiscreteMetric {
    pub total: u32,
}

impl DiscreteMetric {
    pub fn increment(&mut self) {
        self.add(1);
    }

    pub fn add(&mut self, amount: u32) {
        self.total = self.total.saturating_add(amount)
    }
}

impl Metric for DiscreteMetric {
    type Summary = u32;

    fn summarize(&self) -> Self::Summary {
        self.total
    }
}

impl Add for DiscreteMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            total: self.total.saturating_add(rhs.total),
        }
    }
}

/// A metric tracking the maximum and minimum of something.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct ExtremaMetric {
    pub count: u32,
    pub min: f32,
    pub max: f32,
}

impl ExtremaMetric {
    pub fn push(&mut self, sample: f32) {
        if self.count < u32::MAX {
            if self.count == 0 {
                self.min = sample;
                self.max = sample;
            } else {
                self.min = self.min.min(sample);
                self.max = self.max.max(sample);
            }
            self.count += 1;
        }
    }
}

impl Metric for ExtremaMetric {
    type Summary = (f32, f32);

    fn summarize(&self) -> Self::Summary {
        (self.min, self.max)
    }
}

impl Add for ExtremaMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            count: self.count.saturating_add(rhs.count),
            min: self.min.min(rhs.min),
            max: self.max.max(rhs.max),
        }
    }
}

/// A metric tracking the ratio of data satisfying a condition to all data.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct RatioMetric {
    pub total: u32,
    pub count: u32,
}

impl RatioMetric {
    pub fn push(&mut self, condition: bool) {
        if self.total < u32::MAX {
            self.total += 1;
            if condition {
                self.count += 1;
            }
        }
    }

    /// Returns 0 if there are no data.
    fn ratio(&self) -> f32 {
        (self.count as f64 / self.total.max(1) as f64) as f32
    }
}

impl Metric for RatioMetric {
    type Summary = (u32, f32);

    fn summarize(&self) -> Self::Summary {
        (self.total, self.ratio())
    }
}

impl Add for RatioMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            total: self.total.saturating_add(rhs.total),
            count: self.count.saturating_add(rhs.count),
        }
    }
}

/// A metric tracking a continuous value.
/// Can be aggregated by adding all fields.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct ContinuousMetric {
    pub count: u32,
    // These values get large, so use f64 instead of f32.
    pub total: f64,
    pub squared_total: f64,
}

impl ContinuousMetric {
    /// Returns count as a f64, changing a 0 count to 1 to avoid dividing by zero.
    fn non_zero_count(count: u32) -> f64 {
        count.max(1) as f64
    }

    pub fn push(&mut self, sample: f32) {
        if self.count < u32::MAX {
            self.count += 1;
            self.total += sample as f64;
            self.squared_total += (sample as f64).powi(2);
        }
    }

    fn compute_average(count: u32, total: f64) -> f32 {
        (total / Self::non_zero_count(count)) as f32
    }

    fn average(&self) -> f32 {
        Self::compute_average(self.count, self.total)
    }

    fn compute_standard_deviation(count: u32, total: f64, squared_total: f64) -> f32 {
        let non_zero_count = Self::non_zero_count(count);
        ((squared_total / non_zero_count) - (total / non_zero_count).powi(2)).sqrt() as f32
    }

    fn standard_deviation(&self) -> f32 {
        Self::compute_standard_deviation(self.count, self.total, self.squared_total)
    }
}

impl Metric for ContinuousMetric {
    type Summary = (f32, f32);

    fn summarize(&self) -> Self::Summary {
        (self.average(), self.standard_deviation())
    }
}

impl Add for ContinuousMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            count: self.count.saturating_add(rhs.count),
            total: self.total + rhs.total,
            squared_total: self.squared_total + rhs.squared_total,
        }
    }
}

/// A metric combining `ContinuousMetric` and `ExtremaMetric`.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct ContinuousExtremaMetric {
    pub count: u32,
    pub min: f32,
    pub max: f32,
    pub total: f64,
    pub squared_total: f64,
}

impl ContinuousExtremaMetric {
    pub fn push(&mut self, sample: f32) {
        if self.count < u32::MAX {
            if self.count == 0 {
                self.min = sample;
                self.max = sample;
            } else {
                self.min = self.min.min(sample);
                self.max = self.max.max(sample);
            }
            self.total += sample as f64;
            self.squared_total += (sample as f64).powi(2);
            self.count += 1;
        }
    }

    fn average(&self) -> f32 {
        ContinuousMetric::compute_average(self.count, self.total)
    }

    fn standard_deviation(&self) -> f32 {
        ContinuousMetric::compute_standard_deviation(self.count, self.total, self.squared_total)
    }
}

impl Metric for ContinuousExtremaMetric {
    type Summary = (f32, f32, f32, f32);

    fn summarize(&self) -> Self::Summary {
        (
            self.min,
            self.max,
            self.average(),
            self.standard_deviation(),
        )
    }
}

impl Add for ContinuousExtremaMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            count: self.count.saturating_add(rhs.count),
            min: self.min.min(rhs.min),
            max: self.max.max(rhs.max),
            total: self.total + rhs.total,
            squared_total: self.squared_total + rhs.squared_total,
        }
    }
}
