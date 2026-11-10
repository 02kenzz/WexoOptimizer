use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One snapshot rendered by the performance overlay window.
///
/// Every field is measured: `fps` comes from ETW present events and
/// `gpu_percent` from the GPU Engine performance counters. `None` means the
/// metric is unavailable on this machine, never zero-by-default.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct OverlayStats {
    /// Total CPU usage, 0-100.
    pub cpu_percent: f32,
    /// Aggregate GPU utilization, 0-100.
    pub gpu_percent: Option<f32>,
    /// RAM usage, 0-100.
    pub memory_percent: f32,
    #[ts(type = "number")]
    pub memory_used_bytes: u64,
    #[ts(type = "number")]
    pub memory_total_bytes: u64,
    /// Aggregate disk busy time, 0-100.
    pub disk_busy_percent: Option<f32>,
    #[ts(type = "number")]
    pub network_rx_bytes_per_sec: u64,
    #[ts(type = "number")]
    pub network_tx_bytes_per_sec: u64,
    /// Frames per second of the foreground process.
    pub fps: Option<f32>,
    #[ts(type = "number")]
    pub uptime_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_camel_case_with_optional_metrics() {
        let stats = OverlayStats {
            cpu_percent: 12.5,
            gpu_percent: None,
            memory_percent: 40.0,
            memory_used_bytes: 8_000,
            memory_total_bytes: 16_000,
            disk_busy_percent: Some(3.0),
            network_rx_bytes_per_sec: 1_000,
            network_tx_bytes_per_sec: 200,
            fps: Some(143.0),
            uptime_seconds: 600,
        };

        let json = serde_json::to_string(&stats).expect("serialize");
        assert!(json.contains("\"cpuPercent\":12.5"), "{json}");
        assert!(json.contains("\"gpuPercent\":null"), "{json}");
        assert!(json.contains("\"fps\":143.0"), "{json}");
    }
}
