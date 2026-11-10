//! Hardware and OS collectors.
//!
//! Windows-specific access is isolated here so the rest of the workspace stays
//! testable and portable.

pub mod fps;
pub mod gpu;
pub mod overlay;
pub mod pdh;
pub mod sampler;
pub mod thermal;
#[cfg(windows)]
pub mod wmi;
#[cfg(not(windows))]
#[path = "wmi_other.rs"]
pub mod wmi;

pub use overlay::OverlayProbe;
pub use pdh::PdhSampler;
pub use sampler::LiveSampler;
pub use thermal::ThermalSampler;
pub use wmi::{enrich_profile, WmiCache, WmiDisk, WmiSnapshot};

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sysinfo::{Disks, Networks, System};
use wexo_core::{CpuInfo, DiskInfo, DiskKind, MemoryInfo, NetworkInfo, OsInfo, SystemProfile};

/// Minimum spacing between CPU counter reads.
///
/// `refresh_cpu_usage` computes a delta between two reads, so calling it twice
/// within a few milliseconds produces meaningless values (often a spurious
/// 100%). React StrictMode and concurrent queries do exactly that, so the last
/// reading is reused until the interval has elapsed.
const CPU_REFRESH_MIN: Duration = Duration::from_millis(900);

struct SystemState {
    system: System,
    last_cpu_refresh: Option<Instant>,
    last_cpu_usage: f32,
}

/// A source of `SystemProfile` snapshots.
pub trait SystemCollector: Send + Sync {
    fn profile(&self) -> SystemProfile;
}

/// Live collector backed by `sysinfo`.
///
/// The inner `System` is kept alive between calls so CPU usage is computed as a
/// delta between two samples instead of always reporting zero.
///
/// Static hardware details (CPU model, DIMMs, GPU, disk bus) come from a
/// background WMI collection cached in [`WmiCache`]; the first profile may be
/// missing them until that collection finishes.
pub struct LiveCollector {
    system: Mutex<SystemState>,
    wmi: Arc<WmiCache>,
}

impl LiveCollector {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let wmi = Arc::new(WmiCache::new());
        wmi.refresh_async();
        Self {
            system: Mutex::new(SystemState {
                system,
                last_cpu_refresh: None,
                last_cpu_usage: 0.0,
            }),
            wmi,
        }
    }

    /// Forces a synchronous WMI re-collection, e.g. from a "refresh hardware"
    /// command. Blocks for a few hundred milliseconds at most.
    pub fn refresh_hardware(&self) {
        self.wmi.refresh_now();
    }
}

impl Default for LiveCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCollector for LiveCollector {
    fn profile(&self) -> SystemProfile {
        let mut state = match self.system.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Only dynamic fields are refreshed: `refresh_all` would also walk the
        // process list, which is far more expensive than a profile needs.
        let now = Instant::now();
        let cpu_due = state
            .last_cpu_refresh
            .map_or(true, |last| now.duration_since(last) >= CPU_REFRESH_MIN);
        if cpu_due {
            state.system.refresh_cpu_usage();
            state.last_cpu_refresh = Some(now);
        }

        if cpu_due {
            // Read the usage before the frequency refresh (sysinfo 0.33 on
            // Windows reports 100% again afterwards) and keep it, so calls
            // inside CPU_REFRESH_MIN reuse the same reading.
            state.last_cpu_usage = state.system.global_cpu_usage();
            state.system.refresh_cpu_frequency();
        }
        state.system.refresh_memory();

        let system = &state.system;
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
            base_frequency_mhz: 0,
            usage_percent: state.last_cpu_usage,
        };

        let memory = MemoryInfo {
            total_bytes: system.total_memory(),
            used_bytes: system.used_memory(),
            available_bytes: system.available_memory(),
            swap_total_bytes: system.total_swap(),
            swap_used_bytes: system.used_swap(),
            channels: None,
            speed_mhz: 0,
            module_count: 0,
        };

        let disks = collect_disks();
        let networks = collect_networks();
        let os = collect_os();

        let mut profile = SystemProfile {
            cpu,
            memory,
            disks,
            networks,
            gpus: Vec::new(),
            os,
            collected_at_ms: now_ms(),
            uptime_seconds: System::uptime(),
        };

        if let Some(snapshot) = self.wmi.current() {
            enrich_profile(&mut profile, &snapshot);
        }
        profile
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
                bus_type: String::new(),
                smart_status: None,
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
        name: System::name().unwrap_or_else(default_os_name),
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

/// Product name reported by the OS, e.g. `"Windows 11 Home"` or `"Ubuntu"`.
pub fn os_name() -> String {
    System::name().unwrap_or_else(default_os_name)
}

/// Release version reported by the OS, e.g. `"11"` or `"24.04"`.
pub fn os_version() -> String {
    System::os_version().unwrap_or_default()
}

/// Fallback when the platform cannot report a product name.
fn default_os_name() -> String {
    if cfg!(windows) {
        "Windows".to_string()
    } else {
        std::env::consts::OS.to_string()
    }
}

/// Whether the current process can perform privileged optimizations.
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        // SAFETY: `IsUserAnAdmin` has no preconditions and only reads the token.
        unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
    }
    #[cfg(unix)]
    {
        // SAFETY: `geteuid` has no preconditions and only reads the process id.
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(any(windows, unix)))]
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
    fn consecutive_profiles_reuse_the_cpu_reading() {
        let collector = LiveCollector::new();
        let first = collector.profile();
        let second = collector.profile();
        // Two reads milliseconds apart would otherwise produce a spurious 100%.
        assert_eq!(first.cpu.usage_percent, second.cpu.usage_percent);
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
