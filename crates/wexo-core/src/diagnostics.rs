//! Health scoring and bottleneck detection.
//!
//! Everything here is a pure function of a [`SystemProfile`] plus a window of
//! [`LiveMetrics`] samples, so it can be unit-tested without touching Windows.
//! Dynamic rules require several samples and use p95 values: this trades a bit
//! of latency for far fewer false positives.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::metrics::{LiveMetrics, MetricWindow};
use crate::profile::{DiskKind, SystemProfile};

/// Health score per subsystem plus a weighted global score (0-100).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct HealthScore {
    pub global: u8,
    pub cpu: u8,
    pub memory: u8,
    pub disk: u8,
    pub network: u8,
}

impl HealthScore {
    pub fn clamp(value: i32) -> u8 {
        value.clamp(0, 100) as u8
    }

    /// Recomputes the global score from subsystem scores using the documented weights.
    pub fn recompute_global(&mut self) {
        let weighted = f32::from(self.cpu) * 0.30
            + f32::from(self.memory) * 0.25
            + f32::from(self.disk) * 0.30
            + f32::from(self.network) * 0.15;
        self.global = Self::clamp(weighted.round() as i32);
    }
}

impl Default for HealthScore {
    fn default() -> Self {
        Self {
            global: 100,
            cpu: 100,
            memory: 100,
            disk: 100,
            network: 100,
        }
    }
}

/// A detected performance problem with supporting, language-neutral evidence.
///
/// `id` is stable and maps to `insights.rules.<camelCaseId>` in the frontend
/// translations; the numbers live in `evidence` so Rust never ships user-facing
/// prose.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct Bottleneck {
    pub id: String,
    pub component: BottleneckComponent,
    pub severity: Severity,
    pub confidence: Confidence,
    pub evidence: Vec<BottleneckEvidence>,
}

/// One labelled data point backing a [`Bottleneck`].
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct BottleneckEvidence {
    /// Translation key suffix, e.g. `cpuP95` -> `insights.evidence.cpuP95`.
    pub code: String,
    /// Pre-formatted value (units only, no prose), e.g. `"94"` or `"1.2 GB"`.
    pub value: String,
}

impl Bottleneck {
    fn new(
        id: &str,
        component: BottleneckComponent,
        severity: Severity,
        confidence: Confidence,
    ) -> Self {
        Self {
            id: id.to_string(),
            component,
            severity,
            confidence,
            evidence: Vec::new(),
        }
    }

    fn with(mut self, code: &str, value: impl Into<String>) -> Self {
        self.evidence.push(BottleneckEvidence {
            code: code.to_string(),
            value: value.into(),
        });
        self
    }
}

/// Percentiles and coverage for the window a report was computed from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct WindowSummary {
    pub sample_count: u32,
    #[ts(type = "number")]
    pub duration_ms: u64,
    pub cpu_p50: f32,
    pub cpu_p95: f32,
    pub memory_p95: f32,
    pub disk_busy_p95: f32,
    pub disk_queue_p95: f32,
    #[ts(type = "number")]
    pub network_rx_avg: u64,
    #[ts(type = "number")]
    pub network_tx_avg: u64,
}

/// Complete result of a diagnostic run.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct DiagnosticReport {
    pub profile: SystemProfile,
    pub health: HealthScore,
    pub bottlenecks: Vec<Bottleneck>,
    pub window: WindowSummary,
    #[ts(type = "number")]
    pub generated_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum BottleneckComponent {
    Cpu,
    Gpu,
    Memory,
    Disk,
    Network,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

/// Minimum number of samples before dynamic (usage-based) rules are evaluated.
const MIN_SAMPLES: usize = 5;
/// Samples needed before a dynamic finding is reported with high confidence.
const CONFIDENT_SAMPLES: usize = 20;
/// Only the most recent minute is analysed: an old workload or a startup spike
/// must not keep colouring the current health score and findings.
pub const ANALYSIS_WINDOW_MS: u64 = 60_000;

/// Trims `samples` (oldest first) to the analysis window ending at the newest
/// sample. Returns the input untouched when it is empty or already short.
fn recent_samples(samples: &[LiveMetrics]) -> &[LiveMetrics] {
    let Some(last) = samples.last() else {
        return samples;
    };
    let cutoff = last.timestamp_ms.saturating_sub(ANALYSIS_WINDOW_MS);
    let start = samples.partition_point(|sample| sample.timestamp_ms < cutoff);
    &samples[start..]
}

fn dynamic_confidence(sample_count: usize) -> Confidence {
    if sample_count >= CONFIDENT_SAMPLES {
        Confidence::High
    } else {
        Confidence::Medium
    }
}

/// Piecewise-linear score from ascending `(anchor, score)` points.
fn score_curve(value: f32, anchors: &[(f32, f32)]) -> f32 {
    let Some((first_value, first_score)) = anchors.first().copied() else {
        return 100.0;
    };
    if value <= first_value {
        return first_score;
    }
    for pair in anchors.windows(2) {
        let (left_value, left_score) = pair[0];
        let (right_value, right_score) = pair[1];
        if value <= right_value {
            let span = right_value - left_value;
            if span <= f32::EPSILON {
                return right_score;
            }
            let ratio = (value - left_value) / span;
            return left_score + ratio * (right_score - left_score);
        }
    }
    anchors.last().map(|(_, score)| *score).unwrap_or(100.0)
}

fn clamp_score(value: f32) -> u8 {
    HealthScore::clamp(value.round() as i32)
}

fn gib(bytes: u64) -> f64 {
    bytes as f64 / GIB
}

fn percent(value: f32) -> String {
    format!("{value:.0}")
}

/// The disk that hosts Windows, used for capacity and kind checks.
pub fn system_disk(profile: &SystemProfile) -> Option<&crate::profile::DiskInfo> {
    profile
        .disks
        .iter()
        .find(|disk| disk.mount_point.to_uppercase().starts_with("C:"))
        .or_else(|| profile.disks.iter().find(|disk| !disk.is_removable))
}

fn swap_percent(profile: &SystemProfile) -> Option<f32> {
    if profile.memory.swap_total_bytes == 0 {
        return None;
    }
    Some(
        (profile.memory.swap_used_bytes as f64 / profile.memory.swap_total_bytes as f64 * 100.0)
            as f32,
    )
}

pub fn compute_health(profile: &SystemProfile, samples: &[LiveMetrics]) -> HealthScore {
    let window = MetricWindow::new(recent_samples(samples));
    let sample_count = window.len();

    // ---- CPU -------------------------------------------------------------
    let cpu_stats = window.cpu();
    let cpu_reference = cpu_stats
        .map(|stats| stats.p95)
        .unwrap_or(profile.cpu.usage_percent);
    let mut cpu = score_curve(
        cpu_reference,
        &[
            (0.0, 100.0),
            (25.0, 100.0),
            (70.0, 80.0),
            (90.0, 55.0),
            (100.0, 30.0),
        ],
    );
    if sample_count >= MIN_SAMPLES {
        let base = profile.cpu.base_frequency_mhz;
        if base > 0 {
            if let (Some(frequency), Some(stats)) = (window.cpu_frequency(), cpu_stats) {
                if frequency.p50 < base as f32 * 0.80 && stats.p50 >= 40.0 {
                    // The CPU is loaded but running well below its nominal clock.
                    cpu -= 8.0;
                }
            }
        }
    }

    // ---- Memory ----------------------------------------------------------
    let memory_stats = window.memory();
    let memory_reference = memory_stats
        .map(|stats| stats.p95)
        .unwrap_or_else(|| profile.memory.used_percent());
    let mut memory = score_curve(
        memory_reference,
        &[
            (0.0, 100.0),
            (50.0, 100.0),
            (70.0, 85.0),
            (85.0, 70.0),
            (95.0, 48.0),
            (100.0, 35.0),
        ],
    );
    if profile.memory.total_bytes > 0
        && gib(profile.memory.total_bytes) < 8.0
        && memory_reference > 60.0
    {
        memory = memory.min(70.0);
    }
    if let Some(swap) = swap_percent(profile) {
        if swap >= 70.0 {
            memory -= 10.0;
        } else if swap >= 40.0 {
            memory -= 5.0;
        }
    }

    // ---- Disk ------------------------------------------------------------
    let fixed_disks: Vec<_> = profile
        .disks
        .iter()
        .filter(|disk| !disk.is_removable)
        .collect();
    let mut disk = if let Some(system) = system_disk(profile) {
        let free = f64::from(system.free_percent()) as f32;
        score_curve(
            free,
            &[
                (0.0, 15.0),
                (4.0, 35.0),
                (8.0, 60.0),
                (15.0, 80.0),
                (25.0, 100.0),
                (100.0, 100.0),
            ],
        )
    } else if fixed_disks.is_empty() {
        60.0
    } else {
        100.0
    };

    if sample_count >= MIN_SAMPLES {
        if let Some(busy) = window.disk_busy() {
            if busy.p95 >= 95.0 {
                disk -= 25.0;
            } else if busy.p95 >= 85.0 {
                disk -= 15.0;
            } else if busy.p95 >= 70.0 {
                disk -= 8.0;
            }
        }
    }
    if profile
        .disks
        .iter()
        .any(|disk| disk.kind == DiskKind::Hdd && !disk.is_removable)
    {
        disk = disk.min(80.0);
    }

    // ---- Network ---------------------------------------------------------
    // Without link speed there is no honest saturation metric, so the score
    // only reflects whether a usable adapter was detected.
    let network = if profile.networks.is_empty() {
        60.0
    } else {
        100.0
    };

    let mut health = HealthScore {
        global: 0,
        cpu: clamp_score(cpu),
        memory: clamp_score(memory),
        disk: clamp_score(disk),
        network: clamp_score(network),
    };
    health.recompute_global();
    health
}

pub fn analyze(profile: &SystemProfile, samples: &[LiveMetrics]) -> DiagnosticReport {
    let samples = recent_samples(samples);
    let window = MetricWindow::new(samples);
    let sample_count = window.len();
    let cpu_stats = window.cpu();
    let memory_stats = window.memory();
    let disk_busy = window.disk_busy();
    let disk_queue = window.disk_queue();
    let network = window.network_average();

    let mut bottlenecks = Vec::new();

    // ---- Static rules (hardware / capacity) ------------------------------
    let memory_gb = gib(profile.memory.total_bytes);
    if profile.memory.total_bytes > 0 && memory_gb < 8.0 {
        let severity = if memory_gb < 6.0 {
            Severity::Medium
        } else {
            Severity::Low
        };
        bottlenecks.push(
            Bottleneck::new(
                "memory-low-capacity",
                BottleneckComponent::Memory,
                severity,
                Confidence::High,
            )
            .with("totalGb", format!("{memory_gb:.0}")),
        );
    }

    if profile.memory.channels == Some(1) {
        let confidence = if profile.memory.module_count <= 1 {
            Confidence::High
        } else {
            Confidence::Medium
        };
        bottlenecks.push(
            Bottleneck::new(
                "memory-single-channel",
                BottleneckComponent::Memory,
                Severity::Medium,
                confidence,
            )
            .with("modules", profile.memory.module_count.max(1).to_string())
            .with("totalGb", format!("{memory_gb:.0}")),
        );
    }

    for disk in profile.disks.iter().filter(|disk| !disk.is_removable) {
        let free = f64::from(disk.free_percent());
        let is_system = system_disk(profile)
            .map(|system| system.mount_point == disk.mount_point)
            .unwrap_or(false);
        if free < 10.0 {
            let severity = if free < 5.0 && is_system {
                Severity::Critical
            } else {
                Severity::High
            };
            bottlenecks.push(
                Bottleneck::new(
                    "disk-space",
                    BottleneckComponent::Disk,
                    severity,
                    Confidence::High,
                )
                .with("mount", disk.mount_point.clone())
                .with("freeGb", format!("{:.1}", gib(disk.available_bytes)))
                .with("freePercent", format!("{free:.0}")),
            );
        }
    }

    if let Some(system) = system_disk(profile) {
        if system.kind == DiskKind::Hdd {
            bottlenecks.push(
                Bottleneck::new(
                    "disk-hdd-system",
                    BottleneckComponent::Disk,
                    Severity::Medium,
                    Confidence::High,
                )
                .with("mount", system.mount_point.clone()),
            );
        }
    }

    if profile.networks.is_empty() {
        bottlenecks.push(Bottleneck::new(
            "network-no-adapter",
            BottleneckComponent::Network,
            Severity::Low,
            Confidence::High,
        ));
    }

    // ---- Dynamic rules (need a stable window) ----------------------------
    if sample_count >= MIN_SAMPLES {
        let confidence = dynamic_confidence(sample_count);
        let window_sec = window.duration_ms() / 1000;

        if let Some(stats) = cpu_stats {
            if stats.p95 >= 90.0 && stats.p50 >= 60.0 {
                let severity = if stats.p95 >= 98.0 && stats.p50 >= 85.0 {
                    Severity::Critical
                } else {
                    Severity::High
                };
                bottlenecks.push(
                    Bottleneck::new(
                        "cpu-sustained",
                        BottleneckComponent::Cpu,
                        severity,
                        confidence,
                    )
                    .with("cpuP95", percent(stats.p95))
                    .with("cpuP50", percent(stats.p50))
                    .with("samples", sample_count.to_string())
                    .with("windowSec", window_sec.to_string()),
                );
            }

            let base = profile.cpu.base_frequency_mhz;
            if base > 0 {
                if let Some(frequency) = window.cpu_frequency() {
                    if frequency.p50 < base as f32 * 0.75 && stats.p50 >= 40.0 {
                        bottlenecks.push(
                            Bottleneck::new(
                                "cpu-throttle",
                                BottleneckComponent::Cpu,
                                Severity::Medium,
                                Confidence::Medium,
                            )
                            .with("frequencyMhz", format!("{:.0}", frequency.p50))
                            .with("baseFrequencyMhz", base.to_string())
                            .with("cpuP50", percent(stats.p50)),
                        );
                    }
                }
            }
        }

        if let Some(stats) = memory_stats {
            if stats.p95 >= 88.0 {
                let severity = if stats.p95 >= 95.0 {
                    Severity::Critical
                } else {
                    Severity::High
                };
                let available = window
                    .latest()
                    .map(|sample| {
                        sample
                            .memory_total_bytes
                            .saturating_sub(sample.memory_used_bytes)
                    })
                    .filter(|bytes| *bytes > 0)
                    .unwrap_or(profile.memory.available_bytes);
                let mut bottleneck = Bottleneck::new(
                    "memory-pressure",
                    BottleneckComponent::Memory,
                    severity,
                    confidence,
                )
                .with("usedPercent", percent(stats.p95))
                .with("availableGb", format!("{:.1}", gib(available)));
                if let Some(swap) = swap_percent(profile) {
                    bottleneck = bottleneck.with("swapPercent", percent(swap));
                }
                bottlenecks.push(bottleneck);
            }
        }

        let saturated = match (disk_busy, disk_queue) {
            (Some(busy), Some(queue)) => busy.p95 >= 85.0 && queue.p95 >= 1.5,
            (Some(busy), None) => busy.p95 >= 95.0,
            _ => false,
        };
        if saturated {
            let (busy_value, queue_value) = (
                disk_busy.map(|s| s.p95).unwrap_or(0.0),
                disk_queue.map(|s| s.p95).unwrap_or(0.0),
            );
            let severity = if busy_value >= 95.0 && queue_value >= 3.0 {
                Severity::Critical
            } else {
                Severity::High
            };
            bottlenecks.push(
                Bottleneck::new(
                    "disk-saturated",
                    BottleneckComponent::Disk,
                    severity,
                    confidence,
                )
                .with("busyP95", percent(busy_value))
                .with("queueP95", format!("{queue_value:.1}"))
                .with("samples", sample_count.to_string()),
            );
        }
    }

    bottlenecks.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| right.confidence.cmp(&left.confidence))
            .then_with(|| left.id.cmp(&right.id))
    });

    let health = compute_health(profile, samples);

    DiagnosticReport {
        profile: profile.clone(),
        health,
        bottlenecks,
        window: WindowSummary {
            sample_count: sample_count as u32,
            duration_ms: window.duration_ms(),
            cpu_p50: cpu_stats.map(|stats| stats.p50).unwrap_or(0.0),
            cpu_p95: cpu_stats.map(|stats| stats.p95).unwrap_or(0.0),
            memory_p95: memory_stats.map(|stats| stats.p95).unwrap_or(0.0),
            disk_busy_p95: disk_busy.map(|stats| stats.p95).unwrap_or(0.0),
            disk_queue_p95: disk_queue.map(|stats| stats.p95).unwrap_or(0.0),
            network_rx_avg: network.0,
            network_tx_avg: network.1,
        },
        generated_at_ms: now_ms(),
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::LiveMetrics;
    use crate::profile::{CpuInfo, DiskInfo, MemoryInfo, NetworkInfo, OsInfo};

    fn profile() -> SystemProfile {
        SystemProfile {
            cpu: CpuInfo {
                brand: "Test CPU".into(),
                vendor: "test".into(),
                physical_cores: 8,
                logical_cores: 16,
                current_frequency_mhz: 3600,
                base_frequency_mhz: 3600,
                usage_percent: 10.0,
            },
            memory: MemoryInfo {
                total_bytes: 32 * 1024_u64.pow(3),
                used_bytes: 10 * 1024_u64.pow(3),
                available_bytes: 22 * 1024_u64.pow(3),
                swap_total_bytes: 4 * 1024_u64.pow(3),
                swap_used_bytes: 0,
                channels: Some(2),
                speed_mhz: 3200,
                module_count: 2,
            },
            disks: vec![DiskInfo {
                name: "Test NVMe".into(),
                mount_point: "C:\\".into(),
                total_bytes: 1_000 * 1024_u64.pow(3),
                available_bytes: 500 * 1024_u64.pow(3),
                kind: DiskKind::Nvme,
                is_removable: false,
                bus_type: "NVMe".into(),
                smart_status: None,
            }],
            networks: vec![NetworkInfo {
                name: "Ethernet".into(),
                mac_address: "AA:BB:CC:DD:EE:FF".into(),
                received_bytes: 0,
                transmitted_bytes: 0,
            }],
            gpus: Vec::new(),
            os: OsInfo {
                name: "Windows".into(),
                version: "11".into(),
                build: "26100".into(),
                kernel_version: "10.0.26100".into(),
                hostname: "TEST".into(),
                arch: "x86_64".into(),
            },
            collected_at_ms: 1,
            uptime_seconds: 3_600,
        }
    }

    fn sample(timestamp_ms: u64, cpu: f32, memory: f32) -> LiveMetrics {
        LiveMetrics {
            timestamp_ms,
            cpu_percent: cpu,
            memory_percent: memory,
            cpu_frequency_mhz: 3_600,
            memory_used_bytes: 10 * 1024_u64.pow(3),
            memory_total_bytes: 32 * 1024_u64.pow(3),
            disk_busy_percent: Some(5.0),
            disk_queue_length: Some(0.1),
            ..LiveMetrics::empty(timestamp_ms, 3_600)
        }
    }

    fn window(count: usize, cpu: f32, memory: f32) -> Vec<LiveMetrics> {
        (0..count)
            .map(|index| sample(index as u64 * 1_000, cpu, memory))
            .collect()
    }

    #[test]
    fn global_score_is_clamped_and_weighted() {
        let mut score = HealthScore {
            global: 0,
            cpu: 100,
            memory: 50,
            disk: 100,
            network: 0,
        };
        score.recompute_global();
        // 30 + 12.5 + 30 + 0 = 72.5 -> 73
        assert_eq!(score.global, 73);

        assert_eq!(HealthScore::clamp(-5), 0);
        assert_eq!(HealthScore::clamp(150), 100);
    }

    #[test]
    fn severity_is_ordered() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }

    #[test]
    fn idle_system_is_healthy_and_quiet() {
        let report = analyze(&profile(), &window(30, 8.0, 30.0));
        assert!(
            report.health.global >= 90,
            "health = {}",
            report.health.global
        );
        assert!(report.bottlenecks.is_empty(), "{:?}", report.bottlenecks);
        assert_eq!(report.window.sample_count, 30);
        assert_eq!(report.window.duration_ms, 29_000);
    }

    #[test]
    fn sustained_cpu_load_is_detected() {
        let report = analyze(&profile(), &window(30, 96.0, 40.0));
        let cpu = report
            .bottlenecks
            .iter()
            .find(|bottleneck| bottleneck.id == "cpu-sustained")
            .expect("cpu-sustained");
        assert_eq!(cpu.severity, Severity::High);
        assert_eq!(cpu.confidence, Confidence::High);
        assert!(report.health.cpu <= 40);

        let saturated = analyze(&profile(), &window(30, 99.0, 40.0));
        let cpu = saturated
            .bottlenecks
            .iter()
            .find(|bottleneck| bottleneck.id == "cpu-sustained")
            .expect("cpu-sustained");
        assert_eq!(cpu.severity, Severity::Critical);
    }

    #[test]
    fn short_window_lowers_confidence_and_skips_dynamic_rules() {
        let report = analyze(&profile(), &window(3, 99.0, 99.0));
        assert!(!report
            .bottlenecks
            .iter()
            .any(|bottleneck| bottleneck.id == "cpu-sustained"));
        assert!(!report
            .bottlenecks
            .iter()
            .any(|bottleneck| bottleneck.id == "memory-pressure"));
    }

    #[test]
    fn memory_pressure_uses_p95_and_reports_available_memory() {
        let report = analyze(&profile(), &window(30, 10.0, 97.0));
        let memory = report
            .bottlenecks
            .iter()
            .find(|bottleneck| bottleneck.id == "memory-pressure")
            .expect("memory-pressure");
        assert_eq!(memory.severity, Severity::Critical);
        assert!(memory
            .evidence
            .iter()
            .any(|item| item.code == "availableGb" && item.value == "22.0"));
    }

    #[test]
    fn low_disk_space_on_system_disk_is_critical() {
        let mut profile = profile();
        profile.disks[0].available_bytes = 30 * 1024_u64.pow(3); // ~3%
        let report = analyze(&profile, &[]);
        let disk = report
            .bottlenecks
            .iter()
            .find(|bottleneck| bottleneck.id == "disk-space")
            .expect("disk-space");
        assert_eq!(disk.severity, Severity::Critical);
        assert!(report.health.disk < 40);
    }

    #[test]
    fn hdd_system_disk_and_single_channel_are_reported() {
        let mut profile = profile();
        profile.disks[0].kind = DiskKind::Hdd;
        profile.memory.channels = Some(1);
        profile.memory.module_count = 1;
        let report = analyze(&profile, &[]);
        assert!(report
            .bottlenecks
            .iter()
            .any(|bottleneck| bottleneck.id == "disk-hdd-system"));
        let channel = report
            .bottlenecks
            .iter()
            .find(|bottleneck| bottleneck.id == "memory-single-channel")
            .expect("memory-single-channel");
        assert_eq!(channel.confidence, Confidence::High);
    }

    #[test]
    fn saturated_disk_requires_queue_depth() {
        let mut samples = window(30, 20.0, 50.0);
        for sample in &mut samples {
            sample.disk_busy_percent = Some(97.0);
            sample.disk_queue_length = Some(3.5);
        }
        let report = analyze(&profile(), &samples);
        let disk = report
            .bottlenecks
            .iter()
            .find(|bottleneck| bottleneck.id == "disk-saturated")
            .expect("disk-saturated");
        assert_eq!(disk.severity, Severity::Critical);

        // Busy alone is a burst, not a bottleneck.
        let mut burst = window(30, 20.0, 50.0);
        for sample in &mut burst {
            sample.disk_busy_percent = Some(97.0);
            sample.disk_queue_length = Some(0.2);
        }
        let report = analyze(&profile(), &burst);
        assert!(!report
            .bottlenecks
            .iter()
            .any(|bottleneck| bottleneck.id == "disk-saturated"));
    }

    #[test]
    fn throttling_is_detected_below_base_clock() {
        let mut samples = window(30, 65.0, 50.0);
        for sample in &mut samples {
            sample.cpu_frequency_mhz = 2_200;
        }
        let report = analyze(&profile(), &samples);
        assert!(report
            .bottlenecks
            .iter()
            .any(|bottleneck| bottleneck.id == "cpu-throttle"));
    }

    #[test]
    fn report_round_trips_as_json() {
        let report = analyze(&profile(), &window(10, 20.0, 50.0));
        let json = serde_json::to_string(&report).expect("serialize");
        assert!(json.contains("\"generatedAtMs\""));
        assert!(json.contains("\"sampleCount\""));
        let parsed: DiagnosticReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.window.sample_count, 10);
    }

    #[test]
    fn analysis_ignores_samples_older_than_the_window() {
        let mut samples = window(120, 20.0, 50.0);
        // A startup spike 90 s ago must not colour the current report.
        for sample in samples.iter_mut().take(30) {
            sample.cpu_percent = 99.0;
            sample.memory_percent = 99.0;
        }

        let report = analyze(&profile(), &samples);

        assert!(report.window.sample_count <= 61);
        assert!(report.window.duration_ms <= ANALYSIS_WINDOW_MS);
        assert!(report.window.cpu_p95 < 50.0);
        assert!(!report
            .bottlenecks
            .iter()
            .any(|bottleneck| bottleneck.id == "cpu-sustained"));
    }
}
