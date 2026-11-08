use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Structured snapshot of the machine's hardware and OS.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct SystemProfile {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub networks: Vec<NetworkInfo>,
    pub gpus: Vec<GpuInfo>,
    pub os: OsInfo,
    #[ts(type = "number")]
    pub collected_at_ms: u64,
    #[ts(type = "number")]
    pub uptime_seconds: u64,
}

impl SystemProfile {
    /// A minimal, clearly-labelled placeholder used when collection is not available.
    pub fn unavailable() -> Self {
        Self {
            cpu: CpuInfo {
                brand: "Unknown CPU".into(),
                vendor: "unknown".into(),
                physical_cores: 0,
                logical_cores: 0,
                current_frequency_mhz: 0,
                base_frequency_mhz: 0,
                usage_percent: 0.0,
            },
            memory: MemoryInfo {
                total_bytes: 0,
                used_bytes: 0,
                available_bytes: 0,
                swap_total_bytes: 0,
                swap_used_bytes: 0,
                channels: None,
                speed_mhz: 0,
                module_count: 0,
            },
            disks: Vec::new(),
            networks: Vec::new(),
            gpus: Vec::new(),
            os: OsInfo {
                name: "Windows".into(),
                version: String::new(),
                build: String::new(),
                kernel_version: String::new(),
                hostname: String::new(),
                arch: std::env::consts::ARCH.into(),
            },
            collected_at_ms: 0,
            uptime_seconds: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct CpuInfo {
    pub brand: String,
    pub vendor: String,
    pub physical_cores: u32,
    pub logical_cores: u32,
    #[ts(type = "number")]
    pub current_frequency_mhz: u64,
    /// Nominal (non-boost) clock reported by WMI. `0` when unknown.
    #[ts(type = "number")]
    pub base_frequency_mhz: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct MemoryInfo {
    #[ts(type = "number")]
    pub total_bytes: u64,
    #[ts(type = "number")]
    pub used_bytes: u64,
    #[ts(type = "number")]
    pub available_bytes: u64,
    #[ts(type = "number")]
    pub swap_total_bytes: u64,
    #[ts(type = "number")]
    pub swap_used_bytes: u64,
    /// Number of populated memory channels, when it can be detected (WMI).
    pub channels: Option<u8>,
    /// Configured module speed in MHz. `0` when unknown.
    pub speed_mhz: u32,
    /// Number of populated DIMM slots. `0` when unknown.
    pub module_count: u8,
}

impl MemoryInfo {
    /// Used memory as a percentage (0-100), safe for `total_bytes == 0`.
    pub fn used_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f64 / self.total_bytes as f64 * 100.0) as f32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    #[ts(type = "number")]
    pub total_bytes: u64,
    #[ts(type = "number")]
    pub available_bytes: u64,
    pub kind: DiskKind,
    pub is_removable: bool,
    /// Bus reported by WMI/sysinfo, e.g. `"NVMe"`, `"SATA"`, `"USB"`. Empty when unknown.
    pub bus_type: String,
    /// SMART health status from WMI (MSStorageDriver_FailurePredictData). None when unavailable.
    #[ts(type = "\"healthy\" | \"warning\" | \"critical\" | null")]
    pub smart_status: Option<SmartStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum SmartStatus {
    Healthy,
    Warning,
    Critical,
}

impl DiskInfo {
    /// Free space as a percentage (0-100), safe for `total_bytes == 0`.
    pub fn free_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.available_bytes as f64 / self.total_bytes as f64 * 100.0) as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub enum DiskKind {
    Ssd,
    Hdd,
    Nvme,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct NetworkInfo {
    pub name: String,
    pub mac_address: String,
    #[ts(type = "number")]
    pub received_bytes: u64,
    #[ts(type = "number")]
    pub transmitted_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub driver_version: String,
    #[ts(type = "number | null")]
    pub vram_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../../src/types/generated/")]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub build: String,
    pub kernel_version: String,
    pub hostname: String,
    pub arch: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_round_trips_as_json() {
        let profile = SystemProfile::unavailable();
        let json = serde_json::to_string(&profile).expect("serialize");
        let parsed: SystemProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.os.name, "Windows");
        assert!(json.contains("\"collectedAtMs\""));
        assert!(json.contains("\"gpus\""));
    }

    #[test]
    fn percentages_are_safe_for_empty_values() {
        let memory = MemoryInfo {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
            channels: None,
            speed_mhz: 0,
            module_count: 0,
        };
        assert_eq!(memory.used_percent(), 0.0);

        let disk = DiskInfo {
            name: "x".into(),
            mount_point: "C:\\".into(),
            total_bytes: 0,
            available_bytes: 0,
            kind: DiskKind::Unknown,
            is_removable: false,
            bus_type: String::new(),
            smart_status: None,
        };
        assert_eq!(disk.free_percent(), 0.0);
    }

    #[test]
    fn disk_kind_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&DiskKind::Nvme).unwrap(), "\"nvme\"");
    }
}
