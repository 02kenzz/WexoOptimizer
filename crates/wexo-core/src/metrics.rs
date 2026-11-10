//! Live metric samples and window statistics.
//!
//! The monitor service produces one [`LiveMetrics`] sample per tick. The
//! diagnostic engine consumes a window of them and derives percentiles so a
//! single spike does not get mistaken for a sustained bottleneck.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One instantaneous sample of the machine's dynamic state.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct LiveMetrics {
    #[ts(type = "number")]
    pub timestamp_ms: u64,
    /// Total CPU usage, 0-100.
    pub cpu_percent: f32,
    /// Per-logical-core usage, 0-100. Empty when the collector cannot provide it.
    pub per_core_percent: Vec<f32>,
    #[ts(type = "number")]
    pub cpu_frequency_mhz: u64,
    /// CPU temperature in Celsius. `None` when Windows exposes no sensor.
    pub cpu_temp_celsius: Option<f32>,
    /// GPU temperature in Celsius. `None` when the driver does not report it.
    pub gpu_temp_celsius: Option<f32>,
    #[ts(type = "number")]
    pub memory_used_bytes: u64,
    #[ts(type = "number")]
    pub memory_total_bytes: u64,
    /// RAM usage, 0-100.
    pub memory_percent: f32,
    #[ts(type = "number")]
    pub swap_used_bytes: u64,
    #[ts(type = "number")]
    pub swap_total_bytes: u64,
    /// Aggregate disk busy time, 0-100. `None` when PDH is unavailable.
    pub disk_busy_percent: Option<f32>,
    /// Aggregate disk queue length. `None` when PDH is unavailable.
    pub disk_queue_length: Option<f32>,
    #[ts(type = "number")]
    pub disk_read_bytes_per_sec: u64,
    #[ts(type = "number")]
    pub disk_write_bytes_per_sec: u64,
    #[ts(type = "number")]
    pub network_rx_bytes_per_sec: u64,
    #[ts(type = "number")]
    pub network_tx_bytes_per_sec: u64,
    #[ts(type = "number")]
    pub uptime_seconds: u64,
}

impl LiveMetrics {
    /// An all-zero sample carrying only a timestamp and uptime, used when a
    /// collector has nothing to report yet.
    pub fn empty(timestamp_ms: u64, uptime_seconds: u64) -> Self {
        Self {
            timestamp_ms,
            cpu_percent: 0.0,
            per_core_percent: Vec::new(),
            cpu_frequency_mhz: 0,
            cpu_temp_celsius: None,
            gpu_temp_celsius: None,
            memory_used_bytes: 0,
            memory_total_bytes: 0,
            memory_percent: 0.0,
            swap_used_bytes: 0,
            swap_total_bytes: 0,
            disk_busy_percent: None,
            disk_queue_length: None,
            disk_read_bytes_per_sec: 0,
            disk_write_bytes_per_sec: 0,
            network_rx_bytes_per_sec: 0,
            network_tx_bytes_per_sec: 0,
            uptime_seconds,
        }
    }
}

/// Summary statistics for one metric over a sampling window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    pub p50: f32,
    pub p95: f32,
    pub max: f32,
}

/// Nearest-rank percentile over an unsorted slice. Returns `None` when empty.
///
/// Values are sorted in place; callers that need several percentiles should
/// sort once and use [`percentile_sorted`].
pub fn percentile(values: &mut [f32], fraction: f32) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    percentile_sorted(values, fraction)
}

/// Nearest-rank percentile over an already sorted slice.
pub fn percentile_sorted(sorted: &[f32], fraction: f32) -> Option<f32> {
    if sorted.is_empty() {
        return None;
    }
    let fraction = fraction.clamp(0.0, 1.0);
    // Nearest-rank: ceil(fraction * n) with a 1-based rank.
    let rank = ((fraction * sorted.len() as f32).ceil() as usize).max(1);
    sorted.get(rank - 1).copied()
}

/// Computes `p50`, `p95` and `max` for a set of values.
pub fn stats<I>(values: I) -> Option<Stats>
where
    I: IntoIterator<Item = f32>,
{
    let mut collected: Vec<f32> = values
        .into_iter()
        .filter(|value| value.is_finite())
        .collect();
    if collected.is_empty() {
        return None;
    }
    collected.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    Some(Stats {
        p50: percentile_sorted(&collected, 0.50).unwrap_or(0.0),
        p95: percentile_sorted(&collected, 0.95).unwrap_or(0.0),
        max: collected.last().copied().unwrap_or(0.0),
    })
}

/// Convenience accessors over a slice of samples.
#[derive(Debug, Clone, Copy)]
pub struct MetricWindow<'a> {
    samples: &'a [LiveMetrics],
}

impl<'a> MetricWindow<'a> {
    pub fn new(samples: &'a [LiveMetrics]) -> Self {
        Self { samples }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn latest(&self) -> Option<&'a LiveMetrics> {
        self.samples.last()
    }

    pub fn cpu(&self) -> Option<Stats> {
        stats(self.samples.iter().map(|sample| sample.cpu_percent))
    }

    pub fn memory(&self) -> Option<Stats> {
        stats(self.samples.iter().map(|sample| sample.memory_percent))
    }

    pub fn cpu_frequency(&self) -> Option<Stats> {
        stats(
            self.samples
                .iter()
                .map(|sample| sample.cpu_frequency_mhz as f32),
        )
    }

    pub fn disk_busy(&self) -> Option<Stats> {
        stats(
            self.samples
                .iter()
                .filter_map(|sample| sample.disk_busy_percent),
        )
    }

    pub fn disk_queue(&self) -> Option<Stats> {
        stats(
            self.samples
                .iter()
                .filter_map(|sample| sample.disk_queue_length),
        )
    }

    /// Mean network throughput, returned as `(rx, tx)` in bytes per second.
    pub fn network_average(&self) -> (u64, u64) {
        if self.samples.is_empty() {
            return (0, 0);
        }
        let count = self.samples.len() as u128;
        let rx: u128 = self
            .samples
            .iter()
            .map(|sample| u128::from(sample.network_rx_bytes_per_sec))
            .sum();
        let tx: u128 = self
            .samples
            .iter()
            .map(|sample| u128::from(sample.network_tx_bytes_per_sec))
            .sum();
        ((rx / count) as u64, (tx / count) as u64)
    }

    pub fn duration_ms(&self) -> u64 {
        match (self.samples.first(), self.samples.last()) {
            (Some(first), Some(last)) => last.timestamp_ms.saturating_sub(first.timestamp_ms),
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(timestamp_ms: u64, cpu_percent: f32) -> LiveMetrics {
        LiveMetrics {
            timestamp_ms,
            cpu_percent,
            ..LiveMetrics::empty(timestamp_ms, 0)
        }
    }

    #[test]
    fn percentile_uses_nearest_rank() {
        let mut values = [10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(percentile(&mut values, 0.0), Some(10.0));
        assert_eq!(percentile(&mut values, 0.5), Some(30.0));
        assert_eq!(percentile(&mut values, 0.95), Some(50.0));
        assert_eq!(percentile(&mut values, 1.0), Some(50.0));
        assert_eq!(percentile(&mut [], 0.5), None);
    }

    #[test]
    fn stats_ignore_non_finite_values() {
        let result = stats([1.0, f32::NAN, 3.0, f32::INFINITY, 5.0]).expect("stats");
        assert_eq!(result.max, 5.0);
        assert_eq!(result.p50, 3.0);
    }

    #[test]
    fn window_summarizes_samples() {
        let samples = vec![
            sample(1_000, 10.0),
            sample(2_000, 50.0),
            sample(3_000, 90.0),
        ];
        let window = MetricWindow::new(&samples);
        assert_eq!(window.duration_ms(), 2_000);
        assert_eq!(window.cpu().map(|stats| stats.p95), Some(90.0));
        assert_eq!(window.cpu().map(|stats| stats.p50), Some(50.0));
    }

    #[test]
    fn empty_window_is_safe() {
        let window = MetricWindow::new(&[]);
        assert!(window.cpu().is_none());
        assert_eq!(window.network_average(), (0, 0));
        assert_eq!(window.duration_ms(), 0);
    }
}
