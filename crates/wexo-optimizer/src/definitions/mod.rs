//! Optimization definitions with executable apply/verify/rollback functions.

use std::collections::HashMap;
use std::sync::OnceLock;
use wexo_core::{Applicability, Optimization};
use wexo_sysinfo::{is_elevated, SystemCollector};

pub mod builtins;

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
pub fn check_availability(id: &str, spec: &Applicability) -> bool {
    // Check OS build
    if let Some(min_build) = &spec.os_min_build {
        if let Ok(current) = get_os_build() {
            if current < min_build.parse().unwrap_or(0) {
                return false;
            }
        }
    }

    // Check hardware filters (simplified - in M4 the rules engine handles this)
    for filter in &spec.hardware_filters {
        if !evaluate_hardware_filter(filter) {
            return false;
        }
    }

    // Check admin requirement for certain categories
    if matches!(
        id,
        "disable-sysmain" | "disable-diagtrack" | "power-plan-ultimate" | "tcp-autotuning" | "enable-hags" | "ntfs-last-access"
    ) {
        if !is_elevated() {
            return false;
        }
    }

    true
}

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
    // Simplified evaluation - in M4 this uses the rules engine
    if filter.contains("disk.kind == 'nvme'") || filter.contains("disk.kind == 'ssd'") {
        return is_nvme_or_ssd_system_disk();
    }
    if filter.contains("chassis == 'desktop'") {
        return is_desktop();
    }
    if filter.contains("gpu.driver_model == 'wddm2'") {
        return is_wddm2_or_later();
    }
    true
}

fn is_nvme_or_ssd_system_disk() -> bool {
    let collector = wexo_sysinfo::LiveCollector::new();
    let profile = SystemCollector::profile(&collector);
    profile.disks.iter().any(|d| {
        d.mount_point.to_uppercase().starts_with("C:")
            && (d.kind == wexo_core::DiskKind::Nvme || d.kind == wexo_core::DiskKind::Ssd)
    })
}

fn is_desktop() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Control\\SystemInformation", KEY_READ) {
        if let Ok(chassis) = key.get_value::<String, _>("ChassisType") {
            // 3 = Desktop, 6 = Mini Tower, 7 = Tower
            if let Ok(t) = chassis.parse::<u32>() {
                return matches!(t, 3 | 6 | 7 | 15);
            }
        }
    }
    // Fallback: assume desktop if we have a battery
    !has_battery()
}

fn has_battery() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    if let Ok(_key) = hklm.open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Control\\Power\\PowerSettings", KEY_READ) {
        return true; // Simplified
    }
    false
}

fn is_wddm2_or_later() -> bool {
    // Check WDDM version via registry
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers", KEY_READ) {
        if let Ok(version) = key.get_value::<String, _>("WddmVersion") {
            if let Ok(v) = version.parse::<f32>() {
                return v >= 2.0;
            }
        }
    }
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