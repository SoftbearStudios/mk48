// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::ops::Add;

pub trait Metric: Sized + Add + Default {
    type Summary: Serialize + DeserializeOwned;

    // Must be a tuple. First value is most important.
    type DataPoint: Serialize + DeserializeOwned;

    fn summarize(&self) -> Self::Summary;
    fn data_point(&self) -> Self::DataPoint;
}

/// A metric representing something countable.
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscreteMetric {
    #[serde(rename = "t")]
    pub total: u32,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct DiscreteMetricSummary {
    pub total: u32,
}

impl DiscreteMetric {
    pub fn increment(&mut self) {
        self.add_multiple(1);
    }

    pub fn add_multiple(&mut self, amount: u32) {
        self.total = self.total.saturating_add(amount)
    }

    /// Automatically converts to u32.
    pub fn add_length(&mut self, amount: usize) {
        self.add_multiple(amount.min(u32::MAX as usize) as u32)
    }
}

impl Metric for DiscreteMetric {
    type Summary = DiscreteMetricSummary;
    type DataPoint = (u32,);

    fn summarize(&self) -> Self::Summary {
        DiscreteMetricSummary { total: self.total }
    }

    fn data_point(&self) -> Self::DataPoint {
        (self.total,)
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

/// A metric tracking the maximum and minimum of something discrete.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct DiscreteExtremaMetric {
    #[serde(rename = "c")]
    pub count: u32,
    #[serde(rename = "l")]
    pub min: u32,
    #[serde(rename = "h")]
    pub max: u32,
}

impl DiscreteExtremaMetric {
    pub fn push(&mut self, sample: u32) {
        if self.count == 0 {
            self.min = sample;
            self.max = sample;
        } else if self.count < u32::MAX {
            self.min = self.min.min(sample);
            self.max = self.max.max(sample);
            self.count += 1;
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct DiscreteExtremaMetricSummary {
    pub min: f32,
    pub max: f32,
}

impl Metric for DiscreteExtremaMetric {
    type Summary = Self;
    type DataPoint = (u32, u32);

    fn summarize(&self) -> Self::Summary {
        *self
    }

    fn data_point(&self) -> Self::DataPoint {
        (self.min, self.max)
    }
}

impl Add for DiscreteExtremaMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        if self.count == 0 {
            rhs
        } else if rhs.count == 0 {
            self
        } else {
            Self {
                count: self.count.saturating_add(rhs.count),
                min: self.min.min(rhs.min),
                max: self.max.max(rhs.max),
            }
        }
    }
}

/// A metric tracking the maximum and minimum of something.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct ExtremaMetric {
    #[serde(rename = "c")]
    pub count: u32,
    #[serde(rename = "l")]
    pub min: f32,
    #[serde(rename = "h")]
    pub max: f32,
}

impl ExtremaMetric {
    pub fn push(&mut self, sample: f32) {
        if self.count == 0 {
            self.min = sample;
            self.max = sample;
        } else if self.count < u32::MAX {
            self.min = self.min.min(sample);
            self.max = self.max.max(sample);
            self.count += 1;
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct ExtremaMetricSummary {
    pub min: f32,
    pub max: f32,
}

impl Metric for ExtremaMetric {
    type Summary = Self;
    type DataPoint = (f32, f32);

    fn summarize(&self) -> Self::Summary {
        *self
    }

    fn data_point(&self) -> Self::DataPoint {
        (self.min, self.max)
    }
}

impl Add for ExtremaMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        if self.count == 0 {
            rhs
        } else if rhs.count == 0 {
            self
        } else {
            Self {
                count: self.count.saturating_add(rhs.count),
                min: self.min.min(rhs.min),
                max: self.max.max(rhs.max),
            }
        }
    }
}

/// A metric tracking the ratio of data satisfying a condition to all data.
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatioMetric {
    /// Total population size.
    #[serde(rename = "t")]
    pub total: u32,
    /// Number meeting the condition
    #[serde(rename = "c")]
    pub count: u32,
}

impl RatioMetric {
    pub fn push(&mut self, condition: bool) {
        debug_assert!(self.count <= self.total);
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

    fn percent(&self) -> f32 {
        self.ratio() * 100.0
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct RatioMetricSummary {
    percent: f32,
    total: u32,
}

impl Metric for RatioMetric {
    type Summary = RatioMetricSummary;
    type DataPoint = (f32,);

    fn summarize(&self) -> Self::Summary {
        RatioMetricSummary {
            percent: self.percent(),
            total: self.total,
        }
    }

    fn data_point(&self) -> Self::DataPoint {
        (self.percent(),)
    }
}

impl Add for RatioMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let max = u32::MAX - rhs.total;
        Self {
            total: self.total + rhs.total.min(max),
            count: self.count + rhs.count.min(max),
        }
    }
}

/// A metric tracking a continuous value.
/// Can be aggregated by adding all fields.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub struct ContinuousMetric {
    #[serde(rename = "c")]
    pub count: u32,
    // These values get large, so use f64 instead of f32.
    #[serde(rename = "t")]
    pub total: f64,
    #[serde(rename = "s")]
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ContinuousMetricSummary {
    average: f32,
    standard_deviation: f32,
}

impl Metric for ContinuousMetric {
    type Summary = ContinuousMetricSummary;
    type DataPoint = (f32, f32);

    fn summarize(&self) -> Self::Summary {
        ContinuousMetricSummary {
            average: self.average(),
            standard_deviation: self.standard_deviation(),
        }
    }

    fn data_point(&self) -> Self::DataPoint {
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
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuousExtremaMetric {
    #[serde(rename = "c")]
    pub count: u32,
    #[serde(rename = "l")]
    pub min: f32,
    #[serde(rename = "h")]
    pub max: f32,
    #[serde(rename = "t")]
    pub total: f64,
    #[serde(rename = "s")]
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

    /// Automatically converts to float.
    pub fn push_count(&mut self, sample: usize) {
        self.push(sample as f32);
    }

    pub fn average(&self) -> f32 {
        ContinuousMetric::compute_average(self.count, self.total)
    }

    pub fn standard_deviation(&self) -> f32 {
        ContinuousMetric::compute_standard_deviation(self.count, self.total, self.squared_total)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ContinuousExtremaMetricSummary {
    average: f32,
    standard_deviation: f32,
    min: f32,
    max: f32,
}

impl Metric for ContinuousExtremaMetric {
    type Summary = ContinuousExtremaMetricSummary;
    type DataPoint = (f32, f32, f32);

    fn summarize(&self) -> Self::Summary {
        ContinuousExtremaMetricSummary {
            average: self.average(),
            standard_deviation: self.standard_deviation(),
            min: self.min,
            max: self.max,
        }
    }

    fn data_point(&self) -> Self::DataPoint {
        (self.average(), self.min, self.max)
    }
}

impl Add for ContinuousExtremaMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        if self.count == 0 {
            rhs
        } else if rhs.count == 0 {
            self
        } else {
            Self {
                count: self.count.saturating_add(rhs.count),
                min: self.min.min(rhs.min),
                max: self.max.max(rhs.max),
                total: self.total + rhs.total,
                squared_total: self.squared_total + rhs.squared_total,
            }
        }
    }
}

const BUCKET_COUNT: usize = 10;
const BUCKET_SIZE: usize = 1;

#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistogramMetric {
    /// How many samples have value 0.0-9.99, 10.0-19.99, ... ?
    #[serde(rename = "b")]
    buckets: [u32; BUCKET_COUNT],
    /// How many samples have value below the min bucket?
    #[serde(rename = "o")]
    overflow: u32,
    /// How many samples have value above the max bucket?
    #[serde(rename = "u")]
    underflow: u32,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct HistogramMetricSummary {
    /// What percent samples have value 0.0-9.99, 10.0-19.99, ... ?
    buckets: [f32; BUCKET_COUNT],
    /// What percent samples have value below the min bucket?
    overflow: f32,
    /// What percent samples have value above the max bucket?
    underflow: f32,
}

impl HistogramMetric {
    pub fn push(&mut self, sample: f32) {
        if sample < 0.0 {
            self.underflow = self.underflow.saturating_add(1);
        } else if sample > (BUCKET_COUNT * BUCKET_SIZE) as f32 {
            self.overflow = self.overflow.saturating_add(1);
        } else {
            let bucket = ((sample / BUCKET_SIZE as f32) as usize).min(BUCKET_COUNT - 1);
            self.buckets[bucket] = self.buckets[bucket].saturating_add(1);
        }
    }
}

impl Metric for HistogramMetric {
    type Summary = HistogramMetricSummary;
    type DataPoint = ();

    fn summarize(&self) -> Self::Summary {
        let total = self.buckets.iter().sum::<u32>() + self.overflow + self.underflow;
        let to_percent = if total == 0 {
            0f32
        } else {
            100f32 / total as f32
        };
        let mut buckets = [0f32; BUCKET_COUNT];
        for (&a, b) in self.buckets.iter().zip(buckets.iter_mut()) {
            *b = a as f32 * to_percent;
        }
        let overflow = self.overflow as f32 * to_percent;
        let underflow = self.underflow as f32 * to_percent;

        HistogramMetricSummary {
            buckets,
            overflow,
            underflow,
        }
    }

    fn data_point(&self) -> Self::DataPoint {
        ()
    }
}

impl Add for HistogramMetric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut buckets = [0u32; BUCKET_COUNT];
        for ((a, b), c) in self
            .buckets
            .iter()
            .zip(rhs.buckets.iter())
            .zip(buckets.iter_mut())
        {
            *c = a.saturating_add(*b);
        }
        let overflow = self.overflow + rhs.overflow;
        let underflow = self.underflow + rhs.underflow;

        Self {
            buckets,
            overflow,
            underflow,
        }
    }
}
