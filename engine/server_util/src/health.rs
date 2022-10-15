// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::metrics::ContinuousExtremaMetric;
use log::error;
use simple_server_status::SimpleServerStatus;
use std::mem;
use std::time::{Duration, Instant};

/// Keeps track of the "health" of the server.
pub struct Health {
    system: SimpleServerStatus,
    last: Instant,
    /// Cached CPU fraction.
    cpu: f32,
    cpu_steal: f32,
    /// Cached RAM fraction.
    ram: f32,
    swap: f32,
    /// Cached healthy status.
    healthy: bool,
    /// Last tick instant.
    last_tick: Option<Instant>,
    /// Seconds per tick.
    spt: ContinuousExtremaMetric,
    /// Ticks per second.
    tps: ContinuousExtremaMetric,
    /// Ticks in current TPS measurement period.
    ticks: usize,
    /// Start of TPS measurement.
    tps_start: Instant,
}

impl Health {
    /// How long to cache data for (getting data is relatively expensive).
    const CACHE: Duration = Duration::from_secs(30);

    /// Get (possibly cached) cpu usage from 0 to 1.
    pub fn cpu(&mut self) -> f32 {
        self.refresh_if_necessary();
        self.cpu
    }

    /// Get (possibly cached) cpu steal from 0 to 1.
    pub fn cpu_steal(&mut self) -> f32 {
        self.refresh_if_necessary();
        self.cpu_steal
    }

    /// Get (possibly cached) ram usage from 0 to 1.
    pub fn ram(&mut self) -> f32 {
        self.refresh_if_necessary();
        self.ram
    }

    /// Get (possibly cached) bytes/second received.
    pub fn bandwidth_rx(&mut self) -> u64 {
        self.refresh_if_necessary();
        self.system.net_reception_bandwidth().unwrap_or(0)
    }

    /// Get (possibly cached) bytes/second transmitted.
    pub fn bandwidth_tx(&mut self) -> u64 {
        self.refresh_if_necessary();
        self.system.net_transmission_bandwidth().unwrap_or(0)
    }

    /// Get (possibly cached) TCP connection count.
    pub fn connections(&mut self) -> usize {
        self.refresh_if_necessary();
        self.system.tcp_connections().unwrap_or(0)
    }

    /// Call to get average TPS over a large interval.
    /// May be NAN early on.
    pub fn take_tps(&mut self) -> ContinuousExtremaMetric {
        mem::take(&mut self.tps)
    }

    /// Take seconds-per-tick measurements.
    pub fn take_spt(&mut self) -> ContinuousExtremaMetric {
        mem::take(&mut self.spt)
    }

    /// Call every update a.k.a. tick.
    pub fn record_tick(&mut self, tick_period: f32) {
        let now = Instant::now();
        if let Some(last_tick) = self.last_tick {
            let elapsed = now.duration_since(last_tick).as_secs_f32().clamp(0.0, 10.0);
            if elapsed > tick_period * 2.0 {
                error!("long tick lasted: {elapsed:.2}s");
            }
            self.spt.push(elapsed);
        }
        self.last_tick = Some(now);

        let elapsed = now.duration_since(self.tps_start);
        if elapsed >= Duration::from_secs_f32(1.0 - tick_period * 0.5) {
            if elapsed >= Duration::from_secs(1) {
                self.ticks = self.ticks.saturating_add(1);
                self.tps.push(self.ticks as f32);
                self.ticks = 0;
            } else {
                self.tps.push(self.ticks as f32);
                self.ticks = 1;
            }

            self.tps_start = now;
        } else {
            self.ticks = self.ticks.saturating_add(1);
        }
    }

    /// Gets a binary "healthy" status, false if the server isn't doing so well.
    pub fn healthy(&mut self) -> bool {
        self.refresh_if_necessary();
        self.healthy
    }

    fn refresh_if_necessary(&mut self) {
        if self.last.elapsed() <= Self::CACHE {
            return;
        }
        self.last = Instant::now();
        if let Err(e) = self.system.update() {
            error!("error updating health: {:?}", e);
        }

        self.cpu = self.system.cpu_usage().unwrap_or(0.0);
        self.cpu_steal = self.system.cpu_stolen_usage().unwrap_or(0.0);
        self.ram = self.system.ram_usage().unwrap_or(0.0);
        self.swap = self.system.ram_swap_usage().unwrap_or(0.0);

        // Note: Written with the intention that NaN's do not result in unhealthy.
        self.healthy = !((self.cpu + self.cpu_steal).max(self.ram) > 0.8);
    }
}

impl Default for Health {
    fn default() -> Self {
        Self {
            system: SimpleServerStatus::new(),
            last: Instant::now() - Self::CACHE * 2,
            cpu: 0.0,
            cpu_steal: 0.0,
            ram: 0.0,
            swap: 0.0,
            healthy: true,
            ticks: 0,
            last_tick: None,
            spt: ContinuousExtremaMetric::default(),
            tps: ContinuousExtremaMetric::default(),
            tps_start: Instant::now(),
        }
    }
}
