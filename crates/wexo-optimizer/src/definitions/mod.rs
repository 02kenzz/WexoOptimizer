//! Optimization definitions with executable apply/verify/rollback functions.

use std::collections::HashMap;
use std::sync::OnceLock;
use wexo_core::{Applicability, Optimization};
use wexo_sysinfo::SystemCollector;

#[cfg(windows)]
pub mod builtins;
#[cfg(target_os = "linux")]
pub mod builtins_linux;

/// A runnable optimization with executable functions.
pub struct OptimizationDef {
    pub meta: Optimization,
    pub apply_fn: OptimizationFn<Result<String, String>>,
    pub verify_fn: OptimizationFn<Result<wexo_core::VerificationResult, String>>,
    pub rollback_fn: Option<OptimizationFn<Result<(), String>>>,
    pub benchmark_before: Option<OptimizationFn<Result<f64, String>>>,
    pub benchmark_after: Option<OptimizationFn<Result<f64, String>>>,
}

/// Function type for optimization operations.
pub type OptimizationFn<T> = Box<dyn Fn() -> T + Send + Sync>;

impl OptimizationDef {
    /// Creates a builder for an optimization definition.
    pub fn builder(meta: Optimization) -> OptimizationDefBuilder {
        OptimizationDefBuilder {
            meta,
            apply_fn: None,
            verify_fn: None,
            rollback_fn: None,
            benchmark_before: None,
            benchmark_after: None,
        }
    }
}

/// Builder for OptimizationDef.
pub struct OptimizationDefBuilder {
    meta: Optimization,
    apply_fn: Option<OptimizationFn<Result<String, String>>>,
    verify_fn: Option<OptimizationFn<Result<wexo_core::VerificationResult, String>>>,
    rollback_fn: Option<OptimizationFn<Result<(), String>>>,
    benchmark_before: Option<OptimizationFn<Result<f64, String>>>,
    benchmark_after: Option<OptimizationFn<Result<f64, String>>>,
}

impl OptimizationDefBuilder {
    pub fn apply(mut self, f: impl Fn() -> Result<String, String> + Send + Sync + 'static) -> Self {
        self.apply_fn = Some(Box::new(f));
        self
    }

    pub fn verify(mut self, f: impl Fn() -> Result<wexo_core::VerificationResult, String> + Send + Sync + 'static) -> Self {
        self.verify_fn = Some(Box::new(f));
        self
    }

    pub fn rollback(mut self, f: impl Fn() -> Result<(), String> + Send + Sync + 'static) -> Self {
        self.rollback_fn = Some(Box::new(f));
        self
    }

    pub fn benchmark_before(mut self, f: impl Fn() -> Result<f64, String> + Send + Sync + 'static) -> Self {
        self.benchmark_before = Some(Box::new(f));
        self
    }

    pub fn benchmark_after(mut self, f: impl Fn() -> Result<f64, String> + Send + Sync + 'static) -> Self {
        self.benchmark_after = Some(Box::new(f));
        self
    }

    pub fn build(self) -> OptimizationDef {
        OptimizationDef {
            meta: self.meta,
            apply_fn: self.apply_fn.expect("apply function required"),
            verify_fn: self.verify_fn.expect("verify function required"),
            rollback_fn: self.rollback_fn,
            benchmark_before: self.benchmark_before,
            benchmark_after: self.benchmark_after,
        }
    }
}

/// Checks if an optimization is available on the current system.
pub fn check_availability(_id: &str, spec: &Applicability) -> bool {
    // Registry edits are gated by Windows build number; Linux definitions leave
    // `os_min_build` empty because their checks are tool-based.
    #[cfg(windows)]
    if let Some(min_build) = &spec.os_min_build {
        if let Ok(current) = get_os_build() {
            if current < min_build.parse().unwrap_or(0) {
                return false;
            }
        }
    }
    #[cfg(not(windows))]
    let _ = &spec.os_min_build;

    // Hardware/tool filters decide whether the change means anything here.
    for filter in &spec.hardware_filters {
        if !evaluate_hardware_filter(filter) {
            return false;
        }
    }

    true
}

#[cfg(windows)]
fn get_os_build() -> Result<u32, String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey_with_flags("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", KEY_READ)
        .map_err(|e| e.to_string())?;
    let build: String = key
        .get_value("CurrentBuildNumber")
        .map_err(|e| e.to_string())?;
    build.parse::<u32>().map_err(|e| e.to_string())
}

fn evaluate_hardware_filter(filter: &str) -> bool {
    if filter.contains("disk.kind == 'nvme'") || filter.contains("disk.kind == 'ssd'") {
        return is_nvme_or_ssd_system_disk();
    }
    if filter.contains("chassis == 'desktop'") {
        return is_desktop();
    }
    if filter.contains("gpu.driver_model == 'wddm2'") {
        return is_wddm2_or_later();
    }
    if filter.contains("platform.linux") {
        return cfg!(target_os = "linux");
    }
    if filter.contains("desktop.gnome") {
        return has_gnome_interface();
    }
    if filter.contains("service.bluetooth") {
        return has_systemd_unit("bluetooth.service");
    }
    if filter.contains("systemd") {
        return has_systemd();
    }
    if filter.contains("writable.sysctl") {
        return std::path::Path::new("/proc/sys/vm/swappiness").exists();
    }
    true
}

fn is_nvme_or_ssd_system_disk() -> bool {
    let collector = wexo_sysinfo::LiveCollector::new();
    let profile = SystemCollector::profile(&collector);
    profile.disks.iter().any(|d| {
        is_system_mount(&d.mount_point)
            && (d.kind == wexo_core::DiskKind::Nvme || d.kind == wexo_core::DiskKind::Ssd)
    })
}

fn is_system_mount(mount_point: &str) -> bool {
    #[cfg(windows)]
    {
        mount_point.to_uppercase().starts_with("C:")
    }
    #[cfg(not(windows))]
    {
        mount_point == "/"
    }
}

#[cfg(windows)]
fn is_desktop() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Control\\SystemInformation", KEY_READ) {
        if let Ok(chassis) = key.get_value::<u32, _>("ChassisType") {
            // 3-7 = sobremesa/torre, 15/16/23/24 = chasis compactos o de rack
            return matches!(chassis, 3 | 4 | 5 | 6 | 7 | 15 | 16 | 23 | 24);
        }
    }
    // Fallback: assume desktop if we have a battery
    !has_battery()
}

#[cfg(not(windows))]
fn is_desktop() -> bool {
    !has_battery()
}

#[cfg(windows)]
fn has_battery() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    hklm.open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Services\\CmBatt", KEY_READ)
        .is_ok()
}

#[cfg(not(windows))]
fn has_battery() -> bool {
    std::fs::read_dir("/sys/class/power_supply")
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .any(|entry| entry.file_name().to_string_lossy().starts_with("BAT"))
        })
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_wddm2_or_later() -> bool {
    // Check WDDM version via registry
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers", KEY_READ) {
        if let Ok(version) = key.get_value::<u32, _>("WddmVersion") {
            // WDDM 2.0 y posteriores codifican la versión mayor en la palabra alta.
            return version >= 0x2000;
        }
    }
    // Windows moderno usa WDDM 2.x: no ocultar el ajuste si la lectura falla.
    true
}

#[cfg(not(windows))]
fn is_wddm2_or_later() -> bool {
    // WDDM is a Windows concept; elsewhere the filter does not apply.
    true
}

#[cfg(not(windows))]
fn has_gnome_interface() -> bool {
    std::process::Command::new("gsettings")
        .args(["list-schemas"])
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.trim() == "org.gnome.desktop.interface")
        })
        .unwrap_or(false)
}

#[cfg(windows)]
fn has_gnome_interface() -> bool {
    false
}

#[cfg(not(windows))]
fn has_systemd() -> bool {
    std::path::Path::new("/run/systemd/system").is_dir()
}

#[cfg(windows)]
fn has_systemd() -> bool {
    false
}

#[cfg(not(windows))]
fn has_systemd_unit(unit: &str) -> bool {
    std::process::Command::new("systemctl")
        .args(["list-unit-files", unit])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains(unit))
        .unwrap_or(false)
}

#[cfg(windows)]
fn has_systemd_unit(_unit: &str) -> bool {
    false
}

static AVAILABILITY_CACHE: OnceLock<HashMap<String, bool>> = OnceLock::new();

/// Gets cached availability for all registered optimizations.
pub fn get_availability_cache(registry: &crate::engine::OptimizationRegistry) -> HashMap<String, bool> {
    if let Some(cached) = AVAILABILITY_CACHE.get() {
        return cached.clone();
    }

    let mut map = HashMap::new();
    for def in registry.iter() {
        map.insert(def.meta.id.clone(), check_availability(&def.meta.id, &def.meta.applies_to));
    }
    AVAILABILITY_CACHE.set(map.clone()).ok();
    map
}
