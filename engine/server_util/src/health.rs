use log::warn;
use std::process::Command;
use std::time::{Duration, Instant};
use sysinfo::{get_current_pid, ProcessorExt, RefreshKind, System, SystemExt};

/// Keeps track of the "health" of the server.
pub struct Health {
    system: System,
    last: Instant,
    /// Cached CPU fraction.
    cpu: f32,
    /// Cached RAM fraction.
    ram: f32,
    /// Cached connections count.
    connections: usize,
    /// Cached healthy status.
    healthy: bool,
    /// Updates in current period.
    updates: usize,
    /// Start of UPS measurement.
    ups_start: Instant,
    /// Average UPS in previous period.
    ups_previous: Option<f32>,
}

impl Health {
    /// How long to cache data for (getting data is relatively expensive).
    const CACHE: Duration = Duration::from_secs(30);

    /// Minimum time to sample UPS over.
    const UPS_MINIMUM: Duration = Duration::from_secs(10);

    /// Maximum time to sample UPS over.
    const UPS_MAXIMUM: Duration = Duration::from_secs(6 * 3600);

    /// Get (possibly cached) cpu usage from 0 to 1.
    pub fn cpu(&mut self) -> f32 {
        self.refresh_if_necessary();
        self.cpu
    }

    /// Get (possibly cached) ram usage from 0 to 1.
    pub fn ram(&mut self) -> f32 {
        self.refresh_if_necessary();
        self.ram
    }

    /// Get (possibly cached) TCP connection count.
    pub fn connections(&mut self) -> usize {
        self.refresh_if_necessary();
        self.connections
    }

    /// Call to get average UPS over a large interval.
    /// May be NAN early on.
    pub fn ups(&mut self) -> Option<f32> {
        self.record_previous_ups_if_exceeds(Self::UPS_MINIMUM);
        self.ups_previous
    }

    /// Call every update a.k.a. tick.
    pub fn update_ups(&mut self) {
        self.updates = self.updates.saturating_add(1);
        self.record_previous_ups_if_exceeds(Self::UPS_MAXIMUM);
    }

    fn record_previous_ups_if_exceeds(&mut self, maximum_elapsed: Duration) {
        let elapsed = self.ups_start.elapsed();
        if elapsed > maximum_elapsed {
            self.ups_previous = Some(self.updates as f32 / elapsed.as_secs_f32());
            self.updates = 0;
            self.ups_start = Instant::now();
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

        self.cpu = self.compute_cpu();
        self.ram = self.compute_ram();
        match Self::compute_connection_count() {
            Ok(count) => self.connections = count,
            Err(e) => warn!("could not count connections: {}", e),
        }

        // Note: Written with the intention that NaN's do not result in unhealthy.
        self.healthy = !(self.cpu.max(self.ram) > 0.8);
    }

    fn compute_cpu(&mut self) -> f32 {
        self.system.refresh_cpu();
        self.system
            .processors()
            .iter()
            .map(|processor| processor.cpu_usage())
            .sum::<f32>()
            * 0.01
            / self.system.processors().len() as f32
    }

    fn compute_ram(&mut self) -> f32 {
        self.system.refresh_memory();
        self.system.used_memory() as f32 / self.system.total_memory() as f32
    }

    fn compute_connection_count() -> Result<usize, &'static str> {
        let pid = get_current_pid().map_err(|_| "get pid failed")?;
        let output = Command::new("netstat")
            .arg("-ntp")
            .output()
            .map_err(|_| "netstat failed")?;
        let output_str = std::str::from_utf8(&output.stdout).map_err(|_| "netstat invalid utf8")?;
        let pid_string = format!("{}", pid);
        Ok(output_str
            .lines()
            .filter(|&l| l.contains(&pid_string))
            .count())
    }
}

impl Default for Health {
    fn default() -> Self {
        Self {
            system: System::new_with_specifics(RefreshKind::new().with_cpu().with_memory()),
            last: Instant::now() - Self::CACHE * 2,
            cpu: 0.0,
            ram: 0.0,
            connections: 0,
            healthy: true,
            updates: 0,
            ups_start: Instant::now(),
            ups_previous: None,
        }
    }
}
