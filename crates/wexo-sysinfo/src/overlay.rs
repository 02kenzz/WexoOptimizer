//! Overlay probe: the extra measurements the floating window needs on top of
//! the regular monitor samples (GPU utilization and foreground frame rate).

use wexo_core::{LiveMetrics, OverlayStats};

use crate::fps::FpsMeter;
use crate::gpu::{self, GpuSampler};

/// Owns the optional probes. The ETW session is only opened once the overlay is
/// actually enabled, so a user who never turns it on pays nothing.
pub struct OverlayProbe {
    gpu: GpuSampler,
    fps: Option<FpsMeter>,
    /// ETW may be unavailable (no elevation); trying once avoids log spam.
    fps_attempted: bool,
}

impl OverlayProbe {
    pub fn new() -> Self {
        Self {
            gpu: gpu::open(),
            fps: None,
            fps_attempted: false,
        }
    }

    /// Starts the frame-rate meter if it is not running yet.
    pub fn ensure_running(&mut self) {
        if self.fps.is_none() && !self.fps_attempted {
            self.fps_attempted = true;
            self.fps = FpsMeter::start();
        }
    }

    /// Builds one overlay snapshot from the latest monitor sample.
    pub fn sample(&mut self, metrics: &LiveMetrics) -> OverlayStats {
        self.ensure_running();
        OverlayStats {
            cpu_percent: metrics.cpu_percent,
            gpu_percent: self.gpu.sample(),
            memory_percent: metrics.memory_percent,
            memory_used_bytes: metrics.memory_used_bytes,
            memory_total_bytes: metrics.memory_total_bytes,
            disk_busy_percent: metrics.disk_busy_percent,
            network_rx_bytes_per_sec: metrics.network_rx_bytes_per_sec,
            network_tx_bytes_per_sec: metrics.network_tx_bytes_per_sec,
            fps: self.fps.as_ref().and_then(FpsMeter::foreground_fps),
            uptime_seconds: metrics.uptime_seconds,
        }
    }
}

impl Default for OverlayProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_mirrors_the_monitor_and_never_invents_metrics() {
        let mut probe = OverlayProbe::new();
        let metrics = LiveMetrics {
            cpu_percent: 42.0,
            memory_percent: 55.0,
            memory_used_bytes: 5_000,
            memory_total_bytes: 10_000,
            disk_busy_percent: Some(7.0),
            network_rx_bytes_per_sec: 1_200,
            network_tx_bytes_per_sec: 340,
            uptime_seconds: 90,
            ..LiveMetrics::empty(1, 0)
        };

        let stats = probe.sample(&metrics);
        assert_eq!(stats.cpu_percent, 42.0);
        assert_eq!(stats.memory_percent, 55.0);
        assert_eq!(stats.memory_used_bytes, 5_000);
        assert_eq!(stats.disk_busy_percent, Some(7.0));
        assert_eq!(stats.network_rx_bytes_per_sec, 1_200);
        assert_eq!(stats.uptime_seconds, 90);
    }
}
