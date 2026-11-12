//! Static hardware/OS information from WMI.
//!
//! WMI is comparatively slow (~200-800 ms for the whole set), so it is never
//! called from the monitoring loop. [`WmiCache`] collects once in a background
//! thread, keeps the result for a while, and hands out immutable snapshots.
//!
//! Every query is best-effort: a failing class is logged and skipped so a
//! locked-down machine still gets `sysinfo` values instead of an error.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tracing::warn;
use wexo_core::{DiskInfo, DiskKind, GpuInfo, SmartStatus, SystemProfile};

/// How long a collection result is considered fresh.
const SNAPSHOT_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Default)]
pub struct WmiSnapshot {
    pub collected_at_ms: u64,
    pub cpu_name: Option<String>,
    pub cpu_vendor: Option<String>,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub base_frequency_mhz: u64,
    pub memory_channels: Option<u8>,
    pub memory_speed_mhz: u32,
    pub memory_module_count: u8,
    pub disks: Vec<WmiDisk>,
    pub gpus: Vec<GpuInfo>,
    pub os_build: Option<String>,
    pub os_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WmiDisk {
    pub model: String,
    pub size_bytes: u64,
    pub kind: DiskKind,
    pub bus_type: String,
    pub smart_status: Option<SmartStatus>,
}

/// Thread-safe cache of the last successful WMI collection.
pub struct WmiCache {
    snapshot: RwLock<Option<Arc<WmiSnapshot>>>,
    refreshing: AtomicBool,
}

impl WmiCache {
    pub fn new() -> Self {
        Self {
            snapshot: RwLock::new(None),
            refreshing: AtomicBool::new(false),
        }
    }

    /// Returns the cached snapshot, triggering a background refresh when it is
    /// missing or stale. Never blocks on WMI.
    pub fn current(self: &Arc<Self>) -> Option<Arc<WmiSnapshot>> {
        let cached = self.snapshot.read().ok().and_then(|guard| guard.clone());
        let stale = cached
            .as_ref()
            .map(|snapshot| {
                now_ms().saturating_sub(snapshot.collected_at_ms) > SNAPSHOT_TTL.as_millis() as u64
            })
            .unwrap_or(true);

        if stale {
            self.refresh_async();
        }
        cached
    }

    /// Starts a background refresh if none is running.
    pub fn refresh_async(self: &Arc<Self>) {
        if self.refreshing.swap(true, Ordering::AcqRel) {
            return;
        }

        let cache = Arc::clone(self);
        let spawned = std::thread::Builder::new()
            .name("wexo-wmi".to_string())
            .spawn(move || {
                let snapshot = collect();
                if snapshot.cpu_name.is_some() || !snapshot.disks.is_empty() {
                    if let Ok(mut guard) = cache.snapshot.write() {
                        *guard = Some(Arc::new(snapshot));
                    }
                } else {
                    warn!("WMI: no se obtuvo información del hardware");
                }
                cache.refreshing.store(false, Ordering::Release);
            });

        if let Err(error) = spawned {
            warn!(%error, "WMI: no se pudo iniciar el hilo de recolección");
            self.refreshing.store(false, Ordering::Release);
        }
    }

    /// Collects synchronously (used by the "refresh hardware" command). If a
    /// background collection is already running, waits for it briefly.
    pub fn refresh_now(&self) -> Option<Arc<WmiSnapshot>> {
        let deadline = Instant::now() + Duration::from_secs(4);
        while self.refreshing.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(40));
        }
        if self.refreshing.load(Ordering::Acquire) {
            return self.snapshot.read().ok().and_then(|guard| guard.clone());
        }

        self.refreshing.store(true, Ordering::Release);
        let snapshot = collect();
        self.refreshing.store(false, Ordering::Release);

        if snapshot.cpu_name.is_some() || !snapshot.disks.is_empty() {
            let snapshot = Arc::new(snapshot);
            if let Ok(mut guard) = self.snapshot.write() {
                *guard = Some(Arc::clone(&snapshot));
            }
            Some(snapshot)
        } else {
            self.snapshot.read().ok().and_then(|guard| guard.clone())
        }
    }
}

impl Default for WmiCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Merges WMI data into a `sysinfo`-based profile. WMI wins where it is more
/// precise (CPU model, sockets, DIMMs, GPU, OS build, disk bus); sysinfo keeps
/// owning volatile data such as free space and network counters.
pub fn enrich_profile(profile: &mut SystemProfile, snapshot: &WmiSnapshot) {
    if let Some(name) = snapshot
        .cpu_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
    {
        profile.cpu.brand = name.trim().to_string();
    }
    if let Some(vendor) = snapshot
        .cpu_vendor
        .as_deref()
        .filter(|vendor| !vendor.trim().is_empty())
    {
        profile.cpu.vendor = vendor.trim().to_string();
    }
    if snapshot.physical_cores > 0 {
        profile.cpu.physical_cores = snapshot.physical_cores;
    }
    if snapshot.logical_cores > 0 {
        profile.cpu.logical_cores = snapshot.logical_cores;
    }
    if snapshot.base_frequency_mhz > 0 {
        profile.cpu.base_frequency_mhz = snapshot.base_frequency_mhz;
    }

    if snapshot.memory_module_count > 0 {
        profile.memory.module_count = snapshot.memory_module_count;
    }
    if snapshot.memory_channels.is_some() {
        profile.memory.channels = snapshot.memory_channels;
    }
    if snapshot.memory_speed_mhz > 0 {
        profile.memory.speed_mhz = snapshot.memory_speed_mhz;
    }

    for volume in &mut profile.disks {
        if let Some(physical) = match_disk(volume, &snapshot.disks) {
            if physical.kind != DiskKind::Unknown {
                volume.kind = physical.kind;
                if !physical.model.is_empty() {
                    volume.name = physical.model.clone();
                }
            }
            if !physical.bus_type.is_empty() {
                volume.bus_type = physical.bus_type.clone();
            }
            if let Some(status) = physical.smart_status {
                volume.smart_status = Some(match status {
                    SmartStatus::Healthy => SmartStatus::Healthy,
                    SmartStatus::Warning => SmartStatus::Warning,
                    SmartStatus::Critical => SmartStatus::Critical,
                });
            }
        }
    }

    if !snapshot.gpus.is_empty() {
        profile.gpus = snapshot.gpus.clone();
    }
    if let Some(build) = snapshot
        .os_build
        .as_deref()
        .filter(|build| !build.is_empty())
    {
        profile.os.build = build.to_string();
    }
    if let Some(version) = snapshot
        .os_version
        .as_deref()
        .filter(|version| !version.is_empty())
    {
        if profile.os.version.is_empty() {
            profile.os.version = version.to_string();
        }
    }
}

/// Matches a logical volume to a physical disk by capacity. Only an
/// unambiguous match is accepted, and a single-disk machine is trivially
/// unambiguous.
fn match_disk<'a>(volume: &DiskInfo, physical: &'a [WmiDisk]) -> Option<&'a WmiDisk> {
    if physical.len() == 1 {
        return physical.first();
    }

    let tolerance = (volume.total_bytes / 20).max(8 * 1024_u64.pow(3));
    let mut best: Option<(&WmiDisk, u64)> = None;
    let mut second_best_diff: Option<u64> = None;

    for candidate in physical {
        if candidate.size_bytes == 0 || volume.total_bytes == 0 {
            continue;
        }
        let diff = candidate.size_bytes.abs_diff(volume.total_bytes);
        if diff > tolerance {
            continue;
        }
        match best {
            Some((_, best_diff)) if diff >= best_diff => {
                second_best_diff = Some(second_best_diff.map_or(diff, |second| second.min(diff)));
            }
            Some((_, best_diff)) => {
                second_best_diff = Some(best_diff);
                best = Some((candidate, diff));
            }
            None => best = Some((candidate, diff)),
        }
    }

    // Two disks of (almost) the same size: refuse to guess.
    if let (Some((_, best_diff)), Some(second)) = (best, second_best_diff) {
        if second <= best_diff + 1024_u64.pow(3) {
            return None;
        }
    }
    best.map(|(candidate, _)| candidate)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Extracts the channel identifier from an SMBIOS slot label.
///
/// Examples: `"ChannelA-DIMM0"` -> `A`, `"DIMM_A1"` -> `A`,
/// `"DIMM 0"` -> `None` (ambiguous), `"BANK 0"` -> `None`.
fn channel_key(label: &str) -> Option<char> {
    let upper = label.to_ascii_uppercase();
    if let Some(position) = upper.find("CHANNEL") {
        return upper[position + 7..]
            .chars()
            .find(|character| character.is_ascii_alphanumeric());
    }
    for prefix in ["CHA", "CHB", "CHC", "CHD", "CHE", "CHF", "CHG", "CHH"] {
        if let Some(position) = upper.find(prefix) {
            return upper[position + 2..].chars().next();
        }
    }
    if let Some(position) = upper.find("DIMM") {
        return upper[position + 4..]
            .chars()
            .find(|character| character.is_ascii_alphabetic());
    }
    None
}

/// Determines the populated channel count from DIMM slot labels. Returns
/// `None` when the labels do not encode channel information.
fn detect_channels(labels: &[String]) -> Option<u8> {
    if labels.is_empty() {
        return None;
    }
    if labels.len() == 1 {
        return Some(1);
    }

    let mut channels: Vec<char> = labels
        .iter()
        .filter_map(|label| channel_key(label))
        .collect();
    channels.sort_unstable();
    channels.dedup();

    if channels.is_empty() {
        return None;
    }
    Some((channels.len() as u8).min(8))
}

#[cfg(windows)]
mod windows_query {
    use serde::Deserialize;
    use tracing::debug;
    use wmi::{WMIConnection, WMIResult};

    use super::*;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Win32Processor {
        name: Option<String>,
        manufacturer: Option<String>,
        number_of_cores: Option<u32>,
        number_of_logical_processors: Option<u32>,
        max_clock_speed: Option<u32>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Win32PhysicalMemory {
        bank_label: Option<String>,
        device_locator: Option<String>,
        capacity: Option<u64>,
        speed: Option<u32>,
        configured_clock_speed: Option<u32>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Win32VideoController {
        name: Option<String>,
        adapter_compatibility: Option<String>,
        driver_version: Option<String>,
        adapter_ram: Option<u32>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Win32OperatingSystem {
        version: Option<String>,
        build_number: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct MsftPhysicalDisk {
        friendly_name: Option<String>,
        media_type: Option<u16>,
        bus_type: Option<u16>,
        size: Option<u64>,
        spindle_speed: Option<u32>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct MsStorageDriverFailurePredictData {
        instance_name: Option<String>,
        is_failing: Option<bool>,
        reason: Option<u8>,
        predicted_failure: Option<bool>,
    }

    fn query<T>(connection: &WMIConnection, wql: &str, class: &str) -> Vec<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        match connection.raw_query::<T>(wql) {
            Ok(items) => items,
            Err(error) => {
                debug!(%error, class, "WMI query failed");
                Vec::new()
            }
        }
    }

    fn bus_type_label(bus: u16) -> &'static str {
        match bus {
            3 => "ATA",
            6 => "Fibre Channel",
            7 => "USB",
            8 => "RAID",
            10 => "SAS",
            11 => "SATA",
            14 | 15 => "Virtual",
            16 => "Storage Spaces",
            17 => "NVMe",
            18 => "SCM",
            _ => "",
        }
    }

    fn disk_kind(media_type: u16, bus_type: u16, spindle_speed: u32) -> DiskKind {
        if bus_type == 17 {
            return DiskKind::Nvme;
        }
        match media_type {
            3 => DiskKind::Hdd,
            4 | 5 => DiskKind::Ssd,
            _ if spindle_speed > 0 => DiskKind::Hdd,
            _ => DiskKind::Unknown,
        }
    }

    /// Dedicated VRAM reported by each display driver, keyed by `DriverDesc`.
    ///
    /// WMI's `AdapterRAM` is 32-bit, so it caps at 4 GB on larger cards. The
    /// display class registry stores the real size as a 64-bit value next to
    /// the driver description.
    fn registry_vram() -> Vec<(String, u64)> {
        use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, REG_BINARY, REG_DWORD, REG_QWORD};
        use winreg::RegKey;

        const CLASS_KEY: &str =
            r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";

        fn read_bytes(key: &RegKey, name: &str) -> Option<u64> {
            let value = key.get_raw_value(name).ok()?;
            match value.vtype {
                REG_QWORD if value.bytes.len() >= 8 => {
                    Some(u64::from_le_bytes(value.bytes[..8].try_into().ok()?))
                }
                REG_DWORD if value.bytes.len() >= 4 => Some(u64::from(u32::from_le_bytes(
                    value.bytes[..4].try_into().ok()?,
                ))),
                // Some drivers store the same value as raw little-endian bytes.
                REG_BINARY if value.bytes.len() >= 8 => {
                    Some(u64::from_le_bytes(value.bytes[..8].try_into().ok()?))
                }
                REG_BINARY if value.bytes.len() >= 4 => Some(u64::from(u32::from_le_bytes(
                    value.bytes[..4].try_into().ok()?,
                ))),
                _ => None,
            }
        }

        let Ok(class) =
            RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(CLASS_KEY, KEY_READ)
        else {
            return Vec::new();
        };

        let mut entries = Vec::new();
        for name in class.enum_keys().flatten() {
            let Ok(adapter) = class.open_subkey_with_flags(&name, KEY_READ) else {
                continue;
            };
            let desc: String = match adapter.get_value("DriverDesc") {
                Ok(value) => value,
                Err(_) => continue,
            };
            let desc = desc.trim().to_string();
            if desc.is_empty() {
                continue;
            }
            let bytes = read_bytes(&adapter, "HardwareInformation.qwMemorySize")
                .or_else(|| read_bytes(&adapter, "HardwareInformation.MemorySize"))
                .filter(|bytes| *bytes > 0);
            if let Some(bytes) = bytes {
                entries.push((desc, bytes));
            }
        }
        entries
    }

    /// Finds the registry entry for a WMI adapter name. Names usually match
    /// exactly, but a substring check covers driver-added suffixes.
    pub(super) fn matching_vram(entries: &[(String, u64)], name: &str) -> Option<u64> {
        entries
            .iter()
            .find(|(desc, _)| desc.eq_ignore_ascii_case(name))
            .or_else(|| {
                let wanted = name.to_ascii_lowercase();
                entries.iter().find(|(desc, _)| {
                    let desc = desc.to_ascii_lowercase();
                    desc.contains(&wanted) || wanted.contains(&desc)
                })
            })
            .map(|(_, bytes)| *bytes)
    }

    fn query_gpus(connection: &WMIConnection) -> Vec<GpuInfo> {
        let registry = registry_vram();
        query::<Win32VideoController>(
            connection,
            "SELECT Name, AdapterCompatibility, DriverVersion, AdapterRAM FROM Win32_VideoController",
            "Win32_VideoController",
        )
        .into_iter()
        .filter_map(|controller| {
            let name = controller.name.unwrap_or_default();
            let lowered = name.to_ascii_lowercase();
            if name.trim().is_empty()
                || lowered.contains("microsoft basic display")
                || lowered.contains("remote display")
            {
                return None;
            }
            let name = name.trim().to_string();
            // Prefer the 64-bit registry value; AdapterRAM is only a fallback
            // because it cannot represent more than 4 GB.
            let vram_bytes = matching_vram(&registry, &name).or_else(|| {
                controller
                    .adapter_ram
                    .filter(|bytes| *bytes > 0)
                    .map(u64::from)
            });
            Some(GpuInfo {
                name,
                vendor: controller.adapter_compatibility.unwrap_or_default().trim().to_string(),
                driver_version: controller.driver_version.unwrap_or_default().trim().to_string(),
                vram_bytes,
            })
        })
        .collect()
    }

    fn query_disks(storage: Option<&WMIConnection>) -> Vec<WmiDisk> {
        let Some(storage) = storage else {
            return Vec::new();
        };
        let physical_disks: Vec<WmiDisk> = query::<MsftPhysicalDisk>(
            storage,
            "SELECT FriendlyName, MediaType, BusType, Size, SpindleSpeed FROM MSFT_PhysicalDisk",
            "MSFT_PhysicalDisk",
        )
        .into_iter()
        .map(|disk| {
            let media = disk.media_type.unwrap_or(0);
            let bus = disk.bus_type.unwrap_or(0);
            WmiDisk {
                model: disk.friendly_name.unwrap_or_default().trim().to_string(),
                size_bytes: disk.size.unwrap_or(0),
                kind: disk_kind(media, bus, disk.spindle_speed.unwrap_or(0)),
                bus_type: bus_type_label(bus).to_string(),
                smart_status: None,
            }
        })
        .filter(|disk| disk.size_bytes > 0)
        .collect();

        // Query SMART health data
        let mut smart_map = query_smart_health(&storage);
        physical_disks
            .into_iter()
            .map(|mut disk| {
                disk.smart_status = smart_map.remove(&disk.model);
                disk
            })
            .collect()
    }

    fn query_smart_health(storage: &WMIConnection) -> std::collections::HashMap<String, SmartStatus> {
        let mut map = std::collections::HashMap::new();
        for item in query::<MsStorageDriverFailurePredictData>(
            storage,
            "SELECT InstanceName, IsFailing, Reason, PredictedFailure FROM MSStorageDriver_FailurePredictData",
            "MSStorageDriver_FailurePredictData",
        ) {
            if let Some(name) = item.instance_name.filter(|n| !n.trim().is_empty()) {
                let status = match (item.is_failing, item.predicted_failure, item.reason) {
                    (Some(true), _, _) | (_, Some(true), _) => SmartStatus::Critical,
                    (Some(false), Some(false), Some(reason)) if reason > 0 => SmartStatus::Warning,
                    (Some(false), Some(false), _) => SmartStatus::Healthy,
                    _ => SmartStatus::Healthy,
                };
                map.insert(name.trim().to_string(), status);
            }
        }
        map
    }

    pub fn collect() -> WmiSnapshot {
        let connection = match WMIConnection::new() {
            Ok(connection) => connection,
            Err(error) => {
                warn!(%error, "WMI: no se pudo conectar al proveedor local");
                return WmiSnapshot {
                    collected_at_ms: now_ms(),
                    ..WmiSnapshot::default()
                };
            }
        };

        let processors = query::<Win32Processor>(
            &connection,
            "SELECT Name, Manufacturer, NumberOfCores, NumberOfLogicalProcessors, MaxClockSpeed FROM Win32_Processor",
            "Win32_Processor",
        );
        let physical_cores = processors
            .iter()
            .filter_map(|processor| processor.number_of_cores)
            .sum();
        let logical_cores = processors
            .iter()
            .filter_map(|processor| processor.number_of_logical_processors)
            .sum();
        let base_frequency_mhz = processors
            .iter()
            .filter_map(|processor| processor.max_clock_speed)
            .max()
            .unwrap_or(0) as u64;
        let cpu_name = processors
            .first()
            .and_then(|processor| processor.name.clone());
        let cpu_vendor = processors
            .first()
            .and_then(|processor| processor.manufacturer.clone());

        let modules = query::<Win32PhysicalMemory>(
            &connection,
            "SELECT BankLabel, DeviceLocator, Capacity, Speed, ConfiguredClockSpeed FROM Win32_PhysicalMemory",
            "Win32_PhysicalMemory",
        );
        let labels: Vec<String> = modules
            .iter()
            .map(|module| {
                module
                    .device_locator
                    .as_deref()
                    .filter(|label| !label.trim().is_empty())
                    .or(module.bank_label.as_deref())
                    .unwrap_or_default()
                    .to_string()
            })
            .collect();
        let memory_channels = detect_channels(&labels);
        let memory_speed_mhz = modules
            .iter()
            .filter_map(|module| module.configured_clock_speed.or(module.speed))
            .filter(|speed| *speed > 0)
            .max()
            .unwrap_or(0);
        let memory_module_count = modules
            .iter()
            .filter(|module| module.capacity.unwrap_or(0) > 0)
            .count() as u8;

        let gpus = query_gpus(&connection);

        let os = query::<Win32OperatingSystem>(
            &connection,
            "SELECT Version, BuildNumber FROM Win32_OperatingSystem",
            "Win32_OperatingSystem",
        )
        .into_iter()
        .next();

        let storage = match WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage")
        {
            Ok(connection) => Some(connection),
            Err(error) => {
                debug!(%error, "WMI: namespace de almacenamiento no disponible");
                None
            }
        };
        let disks = query_disks(storage.as_ref());

        WmiSnapshot {
            collected_at_ms: now_ms(),
            cpu_name,
            cpu_vendor,
            physical_cores,
            logical_cores,
            base_frequency_mhz,
            memory_channels,
            memory_speed_mhz,
            memory_module_count,
            disks,
            gpus,
            os_build: os.as_ref().and_then(|os| os.build_number.clone()),
            os_version: os.and_then(|os| os.version),
        }
    }

    // Keeps the unused-import warning away on non-Windows builds of the module.
    #[allow(dead_code)]
    fn _assert_connection_error_is_display(error: WMIResult<()>) {
        if let Err(error) = error {
            debug!(%error, "wmi");
        }
    }
}

#[cfg(windows)]
fn collect() -> WmiSnapshot {
    windows_query::collect()
}

#[cfg(not(windows))]
fn collect() -> WmiSnapshot {
    WmiSnapshot {
        collected_at_ms: now_ms(),
        ..WmiSnapshot::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_labels_are_parsed_conservatively() {
        assert_eq!(channel_key("ChannelA-DIMM0"), Some('A'));
        assert_eq!(channel_key("DIMM_A1"), Some('A'));
        assert_eq!(channel_key("DIMM_B2"), Some('B'));
        assert_eq!(channel_key("CHA-DIMM0"), Some('A'));
        assert_eq!(channel_key("DIMM 0"), None);
        assert_eq!(channel_key("BANK 0"), None);
        assert_eq!(channel_key(""), None);
    }

    #[test]
    fn channel_count_is_never_guessed() {
        assert_eq!(detect_channels(&[]), None);
        assert_eq!(detect_channels(&["DIMM_A1".into()]), Some(1));
        assert_eq!(
            detect_channels(&["DIMM_A1".into(), "DIMM_B1".into()]),
            Some(2)
        );
        assert_eq!(
            detect_channels(&[
                "DIMM_A1".into(),
                "DIMM_A2".into(),
                "DIMM_B1".into(),
                "DIMM_B2".into()
            ]),
            Some(2)
        );
        // Ambiguous labels must not produce a (possibly wrong) single-channel
        // claim.
        assert_eq!(detect_channels(&["DIMM 0".into(), "DIMM 1".into()]), None);
    }

    fn volume(total_gb: u64, mount: &str) -> DiskInfo {
        DiskInfo {
            name: mount.to_string(),
            mount_point: mount.to_string(),
            total_bytes: total_gb * 1024_u64.pow(3),
            available_bytes: total_gb * 1024_u64.pow(3) / 2,
            kind: DiskKind::Unknown,
            is_removable: false,
            bus_type: String::new(),
            smart_status: None,
        }
    }

    fn physical(size_gb: u64, kind: DiskKind, model: &str) -> WmiDisk {
        WmiDisk {
            model: model.to_string(),
            size_bytes: size_gb * 1024_u64.pow(3),
            kind,
            bus_type: "NVMe".to_string(),
            smart_status: None,
        }
    }

    #[test]
    fn single_physical_disk_is_applied_to_every_volume() {
        let mut profile = SystemProfile::unavailable();
        profile.disks = vec![volume(500, "C:\\"), volume(1000, "D:\\")];
        let snapshot = WmiSnapshot {
            collected_at_ms: 1,
            disks: vec![physical(1000, DiskKind::Nvme, "Samsung 980 PRO")],
            ..WmiSnapshot::default()
        };

        enrich_profile(&mut profile, &snapshot);
        assert!(profile
            .disks
            .iter()
            .all(|disk| disk.kind == DiskKind::Nvme && disk.bus_type == "NVMe"));
        assert_eq!(profile.disks[0].name, "Samsung 980 PRO");
    }

    #[test]
    fn ambiguous_sizes_are_not_guessed() {
        let mut profile = SystemProfile::unavailable();
        profile.disks = vec![volume(1000, "C:\\")];
        let snapshot = WmiSnapshot {
            collected_at_ms: 1,
            disks: vec![
                physical(1000, DiskKind::Nvme, "A"),
                physical(1000, DiskKind::Ssd, "B"),
            ],
            ..WmiSnapshot::default()
        };

        enrich_profile(&mut profile, &snapshot);
        assert_eq!(profile.disks[0].kind, DiskKind::Unknown);
    }

    #[test]
    fn sizes_within_tolerance_match_the_closest_disk() {
        let mut profile = SystemProfile::unavailable();
        profile.disks = vec![volume(1000, "C:\\")];
        let snapshot = WmiSnapshot {
            collected_at_ms: 1,
            disks: vec![
                physical(500, DiskKind::Nvme, "Small"),
                physical(1000, DiskKind::Ssd, "Big"),
            ],
            ..WmiSnapshot::default()
        };

        enrich_profile(&mut profile, &snapshot);
        assert_eq!(profile.disks[0].kind, DiskKind::Ssd);
        assert_eq!(profile.disks[0].name, "Big");
    }

    #[cfg(windows)]
    #[test]
    fn vram_entries_match_by_name() {
        let entries = vec![
            ("NVIDIA GeForce RTX 3060".to_string(), 12 * 1024_u64.pow(3)),
            ("Intel(R) UHD Graphics 630".to_string(), 1024_u64.pow(3)),
        ];
        assert_eq!(
            windows_query::matching_vram(&entries, "NVIDIA GeForce RTX 3060"),
            Some(12 * 1024_u64.pow(3))
        );
        assert_eq!(
            windows_query::matching_vram(&entries, "nvidia geforce rtx 3060"),
            Some(12 * 1024_u64.pow(3))
        );
        assert_eq!(
            windows_query::matching_vram(&entries, "AMD Radeon RX 6600"),
            None
        );
    }

    #[cfg(windows)]
    #[test]
    fn collects_real_hardware_from_wmi() {
        let snapshot = collect();
        println!("{snapshot:#?}");

        let name = snapshot.cpu_name.as_deref().expect("CPU name from WMI");
        assert!(!name.trim().is_empty());
        assert!(snapshot.physical_cores > 0, "physical cores = 0");
        assert!(
            snapshot.logical_cores >= snapshot.physical_cores,
            "logical {} < physical {}",
            snapshot.logical_cores,
            snapshot.physical_cores
        );
        assert!(
            snapshot.base_frequency_mhz > 0,
            "nominal clock not reported"
        );
        assert!(
            snapshot.memory_module_count > 0,
            "no populated memory modules"
        );
        assert!(!snapshot.disks.is_empty(), "no physical disks from WMI");
        assert!(
            snapshot
                .disks
                .iter()
                .any(|disk| disk.kind != DiskKind::Unknown),
            "disk media type not resolved: {:?}",
            snapshot.disks
        );
        assert!(
            !snapshot.gpus.is_empty(),
            "no GPU from Win32_VideoController"
        );
    }
}
