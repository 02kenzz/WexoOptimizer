//! Hardware and OS collectors.
//!
//! Windows-specific access is isolated here so the rest of the workspace stays
//! testable and portable.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use sysinfo::{Disks, Networks, System};
use wexo_core::{CpuInfo, DiskInfo, DiskKind, MemoryInfo, NetworkInfo, OsInfo, SystemProfile};

/// A source of `SystemProfile` snapshots.
pub trait SystemCollector: Send + Sync {
    fn profile(&self) -> SystemProfile;
}

/// Live collector backed by `sysinfo`.
///
/// The inner `System` is kept alive between calls so CPU usage is computed as a
/// delta between two samples instead of always reporting zero.
pub struct LiveCollector {
    system: Mutex<System>,
}

impl LiveCollector {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self {
            system: Mutex::new(system),
        }
    }
}

impl Default for LiveCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCollector for LiveCollector {
    fn profile(&self) -> SystemProfile {
        let mut system = match self.system.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        system.refresh_all();

        let cpus = system.cpus();
        let first_cpu = cpus.first();
        let logical_cores = cpus.len() as u32;

        let cpu = CpuInfo {
            brand: first_cpu
                .map(|cpu| cpu.brand().trim().to_string())
                .filter(|brand| !brand.is_empty())
                .unwrap_or_else(|| "Unknown CPU".to_string()),
            vendor: first_cpu
                .map(|cpu| cpu.vendor_id().to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            physical_cores: system
                .physical_core_count()
                .unwrap_or(logical_cores as usize)
                .max(1) as u32,
            logical_cores,
            current_frequency_mhz: first_cpu.map(|cpu| cpu.frequency()).unwrap_or(0),
            usage_percent: system.global_cpu_usage(),
        };

        let memory = MemoryInfo {
            total_bytes: system.total_memory(),
            used_bytes: system.used_memory(),
            available_bytes: system.available_memory(),
            swap_total_bytes: system.total_swap(),
            swap_used_bytes: system.used_swap(),
            channels: None,
        };

        let disks = collect_disks();
        let networks = collect_networks();
        let os = collect_os();

        SystemProfile {
            cpu,
            memory,
            disks,
            networks,
            os,
            collected_at_ms: now_ms(),
        }
    }
}

fn collect_disks() -> Vec<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|disk| {
            let name = disk.name().to_string_lossy().to_string();
            DiskInfo {
                kind: classify_disk(&name, disk.kind()),
                name,
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total_bytes: disk.total_space(),
                available_bytes: disk.available_space(),
                is_removable: disk.is_removable(),
            }
        })
        .collect()
}

/// Best-effort classification. `sysinfo` does not distinguish NVMe from SATA
/// SSDs, so we use the device name as a hint until the WMI collector lands (M1).
fn classify_disk(name: &str, kind: sysinfo::DiskKind) -> DiskKind {
    let lowered = name.to_ascii_lowercase();
    if lowered.contains("nvme") {
        return DiskKind::Nvme;
    }
    match kind {
        sysinfo::DiskKind::SSD => DiskKind::Ssd,
        sysinfo::DiskKind::HDD => DiskKind::Hdd,
        _ => DiskKind::Unknown,
    }
}

fn collect_networks() -> Vec<NetworkInfo> {
    let networks = Networks::new_with_refreshed_list();
    networks
        .iter()
        .map(|(name, data)| NetworkInfo {
            name: name.clone(),
            mac_address: data.mac_address().to_string(),
            received_bytes: data.total_received(),
            transmitted_bytes: data.total_transmitted(),
        })
        .collect()
}

fn collect_os() -> OsInfo {
    OsInfo {
        name: System::name().unwrap_or_else(|| "Windows".to_string()),
        version: System::os_version().unwrap_or_default(),
        build: String::new(),
        kernel_version: System::kernel_version().unwrap_or_default(),
        hostname: System::host_name().unwrap_or_default(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Windows product name, e.g. `"Windows 11 Home"`.
pub fn os_name() -> String {
    System::name().unwrap_or_else(|| "Windows".to_string())
}

/// Windows release version, e.g. `"11"` or `"10"`.
pub fn os_version() -> String {
    System::os_version().unwrap_or_default()
}

/// Whether the current process can perform privileged optimizations.
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        // SAFETY: `IsUserAnAdmin` has no preconditions and only reads the token.
        unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_collector_returns_sane_values() {
        let collector = LiveCollector::new();
        let profile = collector.profile();
        assert!(!profile.cpu.brand.is_empty());
        assert!(profile.cpu.logical_cores > 0);
        assert!(profile.memory.total_bytes > 0);
        assert!(profile.collected_at_ms > 0);
    }

    #[test]
    fn nvme_is_detected_from_device_name() {
        assert_eq!(
            classify_disk("Samsung SSD 980 PRO NVMe", sysinfo::DiskKind::SSD),
            DiskKind::Nvme
        );
        assert_eq!(
            classify_disk("ST2000DM008", sysinfo::DiskKind::HDD),
            DiskKind::Hdd
        );
    }
}
