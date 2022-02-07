use std::time::{Duration, Instant};
use sysinfo::{ProcessorExt, RefreshKind, System, SystemExt};

/// Keeps track of the "health" of the server.
pub struct Health {
    system: System,
    last: Instant,
    cpu: f32,
    ram: f32,
}

impl Health {
    /// How long to cache data for (getting data is relatively expensive).
    const CACHE: Duration = Duration::from_secs(30);

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

    /// Gets a binary "healthy" status, false if the server isn't doing so well.
    pub fn healthy(&mut self) -> bool {
        self.refresh_if_necessary();
        !(self.cpu.max(self.ram) > 0.8)
    }

    fn refresh_if_necessary(&mut self) {
        if self.last.elapsed() <= Self::CACHE {
            return;
        }
        self.last = Instant::now();
        self.system.refresh_cpu();
        self.system.refresh_memory();
        self.ram = self.system.used_memory() as f32 / self.system.total_memory() as f32;
        self.cpu = self
            .system
            .processors()
            .iter()
            .map(|processor| processor.cpu_usage())
            .sum::<f32>()
            * 0.01
            / self.system.processors().len() as f32;
    }
}

impl Default for Health {
    fn default() -> Self {
        Self {
            system: System::new_with_specifics(RefreshKind::new().with_cpu().with_memory()),
            last: Instant::now() - Self::CACHE * 2,
            cpu: 0.0,
            ram: 0.0,
        }
    }
}
