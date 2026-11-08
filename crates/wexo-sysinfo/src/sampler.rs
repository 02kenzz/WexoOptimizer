//! Live metric sampler.
//!
//! One instance is kept alive by the monitor thread: `sysinfo` needs the
//! previous state to compute CPU percentages and network deltas, and PDH needs
//! two collections before rate counters become valid.
//!
//! Cost per sample is intentionally small (~1-3 ms): CPU usage, memory and
//! network counters only. Process enumeration is never done here.

use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use sysinfo::{Networks, System};
use wexo_core::LiveMetrics;

use crate::pdh::{try_open, PdhSampler};

struct NetworkBaseline {
    at: Instant,
    totals: HashMap<String, (u64, u64)>,
}

/// Stateful sampler that produces one [`LiveMetrics`] per call.
pub struct LiveSampler {
    system: System,
    networks: Networks,
    pdh: Option<PdhSampler>,
    baseline: Option<NetworkBaseline>,
}

impl LiveSampler {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_usage();
        system.refresh_memory();

        let mut networks = Networks::new_with_refreshed_list();
        networks.refresh(true);

        Self {
            system,
            networks,
            pdh: try_open(),
            baseline: None,
        }
    }

    /// Samples dynamic metrics. Percentages are 0-100 and rates are per second.
    pub fn sample(&mut self) -> LiveMetrics {
        self.system.refresh_cpu_usage();
        // Cheap single syscall, and without it `cpu.frequency()` stays at 0
        // because only the usage counters were refreshed.
        self.system.refresh_cpu_frequency();
        self.system.refresh_memory();
        self.networks.refresh(true);

        let pdh = self
            .pdh
            .as_mut()
            .map(|sampler| sampler.sample())
            .unwrap_or_default();

        let cpus = self.system.cpus();
        let per_core_percent: Vec<f32> = cpus.iter().map(|cpu| cpu.cpu_usage()).collect();
        let cpu_frequency_mhz = cpus.first().map(|cpu| cpu.frequency()).unwrap_or(0);

        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        let memory_percent = if total_memory == 0 {
            0.0
        } else {
            (used_memory as f64 / total_memory as f64 * 100.0) as f32
        };

        let (rx_bytes_per_sec, tx_bytes_per_sec) = self.network_rates(Instant::now());
        let total_cpu = pdh
            .cpu_percent
            .unwrap_or_else(|| self.system.global_cpu_usage())
            .clamp(0.0, 100.0);

        LiveMetrics {
            timestamp_ms: now_ms(),
            cpu_percent: total_cpu,
            per_core_percent,
            cpu_frequency_mhz,
            memory_used_bytes: used_memory,
            memory_total_bytes: total_memory,
            memory_percent,
            swap_used_bytes: self.system.used_swap(),
            swap_total_bytes: self.system.total_swap(),
            disk_busy_percent: pdh.disk_busy_percent,
            disk_queue_length: pdh.disk_queue_length,
            disk_read_bytes_per_sec: pdh.disk_read_bytes_per_sec,
            disk_write_bytes_per_sec: pdh.disk_write_bytes_per_sec,
            network_rx_bytes_per_sec: rx_bytes_per_sec,
            network_tx_bytes_per_sec: tx_bytes_per_sec,
            uptime_seconds: System::uptime(),
        }
    }

    /// Sums per-interface deltas. Interfaces seen for the first time are
    /// skipped, so plugging in a USB NIC cannot produce a fake throughput spike.
    fn network_rates(&mut self, now: Instant) -> (u64, u64) {
        let mut totals = HashMap::with_capacity(self.networks.len());
        let (mut rx, mut tx) = (0u64, 0u64);

        for (name, data) in self.networks.iter() {
            if name.to_ascii_lowercase().contains("loopback") {
                continue;
            }
            let current = (data.total_received(), data.total_transmitted());
            totals.insert(name.clone(), current);

            if let Some(baseline) = &self.baseline {
                if let Some(previous) = baseline.totals.get(name) {
                    rx = rx.saturating_add(current.0.saturating_sub(previous.0));
                    tx = tx.saturating_add(current.1.saturating_sub(previous.1));
                }
            }
        }

        let elapsed_ms = self
            .baseline
            .as_ref()
            .map(|baseline| now.saturating_duration_since(baseline.at).as_millis())
            .unwrap_or(0);

        self.baseline = Some(NetworkBaseline { at: now, totals });

        if elapsed_ms < 100 {
            return (0, 0);
        }
        (
            ((u128::from(rx) * 1_000) / elapsed_ms) as u64,
            ((u128::from(tx) * 1_000) / elapsed_ms) as u64,
        )
    }
}

impl Default for LiveSampler {
    fn default() -> Self {
        Self::new()
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;

    use super::*;

    #[test]
    fn samples_are_sane_and_cheap() {
        let mut sampler = LiveSampler::new();
        // Let sysinfo accumulate a CPU delta and PDH produce rate counters.
        sleep(Duration::from_millis(250));

        let mut worst = Duration::ZERO;
        let mut last = None;
        for _ in 0..5 {
            let started = Instant::now();
            let sample = sampler.sample();
            worst = worst.max(started.elapsed());
            last = Some(sample);
            sleep(Duration::from_millis(220));
        }

        let sample = last.expect("sample");
        assert!(sample.memory_total_bytes > 0);
        assert!(sample.memory_used_bytes <= sample.memory_total_bytes);
        assert!(!sample.per_core_percent.is_empty());
        assert!((0.0..=100.0).contains(&sample.cpu_percent));
        assert!((0.0..=100.0).contains(&sample.memory_percent));
        assert!(sample.timestamp_ms > 0);
        assert!(sample.uptime_seconds > 0);

        // Generous bound: catches accidental O(n) work such as process scans,
        // while staying stable on slow CI machines.
        assert!(
            worst < Duration::from_millis(60),
            "worst sample = {worst:?}"
        );
    }

    #[test]
    fn first_network_sample_has_no_rate() {
        let mut sampler = LiveSampler::new();
        let first = sampler.sample();
        assert_eq!(first.network_rx_bytes_per_sec, 0);
        assert_eq!(first.network_tx_bytes_per_sec, 0);
    }
}
