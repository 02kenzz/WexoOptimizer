//! Optimization execution engine: apply, verify, rollback.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tracing::{error, info, warn};
use wexo_core::{Category, SnapshotSpec, VerificationResult};
use wexo_safety::{Snapshot, TransactionError, TransactionLog, TransactionRecord, TransactionStatus};

use crate::definitions::OptimizationDef;

/// Errors during optimization execution.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Preflight failed: {0}")]
    Preflight(String),
    #[error("Optimization not found: {0}")]
    NotFound(String),
    #[error("Snapshot error: {0}")]
    Snapshot(#[from] wexo_safety::SnapshotError),
    #[error("Transaction error: {0}")]
    Transaction(#[from] TransactionError),
    #[error("Optimization failed: {0}")]
    Execution(String),
    #[error("Verification failed: {0}")]
    Verification(String),
    #[error("Rollback failed: {0}")]
    Rollback(String),
}

/// Result of applying optimizations.
#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub transaction: TransactionRecord,
    pub results: Vec<OptimizationResult>,
}

/// Result for a single optimization.
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub optimization_id: String,
    pub applied: bool,
    pub verification: VerificationResult,
    pub message: Option<String>,
    pub benchmark_delta: Option<wexo_core::BenchmarkDelta>,
}

/// Registry of executable optimization definitions.
#[derive(Default)]
pub struct OptimizationRegistry {
    defs: HashMap<String, OptimizationDef>,
}

impl OptimizationRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_builtins();
        registry.refresh_availability();
        registry
    }

    /// Recomputes which optimizations can run on this machine.
    fn refresh_availability(&mut self) {
        for def in self.defs.values_mut() {
            def.meta.available =
                crate::definitions::check_availability(&def.meta.id, &def.meta.applies_to);
        }
    }

    fn register_builtins(&mut self) {
        #[cfg(windows)]
        {
            use crate::definitions::builtins::*;

            self.register(disable_sysmain());
            self.register(disable_diagtrack());
            self.register(power_plan_ultimate());
            self.register(tcp_autotuning());
            self.register(enable_hags());
            self.register(reduce_visual_effects());
            self.register(ntfs_last_access());
        }

        #[cfg(target_os = "linux")]
        {
            use crate::definitions::builtins_linux::*;

            self.register(reduce_visual_effects());
            self.register(disable_bluetooth());
            self.register(vm_swappiness());
        }
    }

    pub fn register(&mut self, def: OptimizationDef) {
        self.defs.insert(def.meta.id.clone(), def);
    }

    pub fn get(&self, id: &str) -> Option<&OptimizationDef> {
        self.defs.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &OptimizationDef> {
        self.defs.values()
    }

    pub fn by_category(&self, category: Category) -> impl Iterator<Item = &OptimizationDef> {
        let category = Some(category);
        self.defs.values().filter(move |d| Some(d.meta.category) == category)
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }
}

/// Core engine that executes optimizations with safety guarantees.
pub struct OptimizationEngine {
    registry: OptimizationRegistry,
    transaction_log: Arc<Mutex<TransactionLog>>,
}

impl OptimizationEngine {
    pub fn new(transaction_log: TransactionLog) -> Self {
        Self {
            registry: OptimizationRegistry::new(),
            transaction_log: Arc::new(Mutex::new(transaction_log)),
        }
    }

    /// Applies a list of optimizations atomically.
    ///
    /// Each optimization gets its own snapshot. If any fails, already-applied
    /// optimizations are rolled back automatically.
    pub fn apply(
        &self,
        optimization_ids: &[String],
        dry_run: bool,
    ) -> Result<ApplyResult, EngineError> {
        // A dry run writes nothing, so it is never blocked; the real apply still
        // refuses to run without room to write a snapshot.
        let preflight = self.preflight()?;
        if !dry_run && preflight.free_disk_bytes < wexo_safety::MIN_FREE_DISK_BYTES {
            return Err(EngineError::Preflight("LowDiskSpace".to_string()));
        }
        let pending_reboot = preflight.pending_reboot;

        // Validate all optimizations exist and are available
        let defs: Vec<&OptimizationDef> = optimization_ids
            .iter()
            .map(|id| {
                self.registry
                    .get(id)
                    .ok_or_else(|| EngineError::NotFound(id.clone()))
                    .and_then(|def| {
                        if def.meta.available {
                            Ok(def)
                        } else {
                            Err(EngineError::Execution(format!("Optimization not available on this system: {}", id)))
                        }
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        // A pending restart only holds back the tweaks that need that restart.
        if !dry_run && pending_reboot {
            if let Some(def) = defs.iter().find(|def| def.meta.requires_reboot) {
                return Err(EngineError::Preflight(format!("PendingReboot: {}", def.meta.id)));
            }
        }

        // Elevation is only required by the tweaks that write machine-wide state.
        if !preflight.is_admin {
            if let Some(def) = defs.iter().find(|def| def.meta.requires_admin) {
                return Err(EngineError::Preflight(format!(
                    "This optimization requires administrator privileges: {}",
                    def.meta.id
                )));
            }
        }

        // Check for conflicts
        for def in &defs {
            for conflict in &def.meta.applies_to.conflicts_with {
                if optimization_ids.contains(conflict) {
                    return Err(EngineError::Execution(format!(
                        "Conflicting optimizations selected: {} and {}",
                        def.meta.id, conflict
                    )));
                }
            }
        }

        // Capture snapshot for all optimizations combined
        let combined_spec = combine_snapshots(&defs.iter().map(|d| &d.meta.snapshot).cloned().collect::<Vec<_>>());
        let snapshot = if !dry_run {
            let log = self.transaction_log.lock();
            log.capture_snapshot(&combined_spec, optimization_ids)?
        } else {
            wexo_safety::Snapshot {
                id: "dry-run".to_string(),
                timestamp_ms: 0,
                optimization_ids: Vec::new(),
                registry_values: HashMap::new(),
                service_states: HashMap::new(),
                file_backups: HashMap::new(),
                absent_files: Vec::new(),
            }
        };

        // Dry runs must not pollute the transaction log.
        let transaction = if dry_run {
            TransactionRecord {
                id: "dry-run".to_string(),
                timestamp_ms: 0,
                optimization_ids: optimization_ids.to_vec(),
                status: TransactionStatus::Applied,
                rollback_available: false,
                message: Some("dry-run".to_string()),
                snapshot_id: None,
            }
        } else {
            self.transaction_log.lock().record(
                optimization_ids,
                TransactionStatus::Applied,
                true,
                None,
                Some(snapshot.id.clone()),
            )?
        };

        // Apply each optimization, undoing everything if one of them fails.
        let mut results = Vec::new();
        for def in defs {
            match self.apply_single(def, dry_run) {
                Ok(result) => results.push(result),
                Err(error) => {
                    if !dry_run {
                        if let Err(rollback_error) = self.rollback_all(&snapshot, &results) {
                            error!(%rollback_error, "rollback after failure failed");
                        }
                        let _ = self
                            .transaction_log
                            .lock()
                            .update_status(&transaction.id, TransactionStatus::Failed);
                    }
                    return Err(error);
                }
            }
        }

        // Verify all
        let mut all_verified = true;
        for result in &results {
            if result.verification != VerificationResult::Verified {
                all_verified = false;
            }
        }

        // If any failed verification and not dry_run, rollback everything
        if !all_verified && !dry_run {
            warn!("Some optimizations failed verification, rolling back");
            self.rollback_all(&snapshot, &results)?;
            // Update transaction status
            let log = self.transaction_log.lock();
            log.update_status(&transaction.id, TransactionStatus::Failed)?;
            return Err(EngineError::Verification("One or more optimizations failed verification".to_string()));
        }

        info!(count = optimization_ids.len(), "optimizations applied successfully");
        Ok(ApplyResult { transaction, results })
    }

    /// Applies a single optimization.
    fn apply_single(&self, def: &OptimizationDef, dry_run: bool) -> Result<OptimizationResult, EngineError> {
        info!(id = %def.meta.id, "applying optimization");

        let mut message = None;

        // Apply the optimization
        if !dry_run {
            match (def.apply_fn)() {
                Ok(msg) => message = Some(msg),
                Err(e) => return Err(EngineError::Execution(e)),
            }
        }

        // Verify
        let verification = if !dry_run {
            match (def.verify_fn)() {
                Ok(v) => v,
                Err(e) => return Err(EngineError::Verification(e)),
            }
        } else {
            VerificationResult::Verified
        };

        Ok(OptimizationResult {
            optimization_id: def.meta.id.clone(),
            applied: !dry_run,
            verification,
            message,
            benchmark_delta: None,
        })
    }

    /// Rolls back all optimizations from a snapshot.
    fn rollback_all(&self, snapshot: &Snapshot, results: &[OptimizationResult]) -> Result<(), EngineError> {
        // Rollback in reverse order
        for result in results.iter().rev() {
            if result.applied {
                if let Some(def) = self.registry.get(&result.optimization_id) {
                    if let Some(rollback_fn) = &def.rollback_fn {
                        if let Err(e) = rollback_fn() {
                            error!(id = %result.optimization_id, %e, "rollback failed");
                        }
                    }
                }
            }
        }

        // Restore snapshot
        wexo_safety::restore_snapshot(snapshot)?;
        Ok(())
    }

    /// Rolls back a specific transaction.
    pub fn rollback_transaction(&self, transaction_id: &str) -> Result<(), EngineError> {
        let log = self.transaction_log.lock();
        let transaction = log.get(transaction_id)?.ok_or_else(|| EngineError::NotFound(transaction_id.to_string()))?;

        if !transaction.rollback_available {
            return Err(EngineError::Rollback("Transaction cannot be rolled back".to_string()));
        }

        // Undo each tweak first: the snapshot afterwards restores every value that
        // already existed, so nothing the tweak created is left behind.
        for id in &transaction.optimization_ids {
            if let Some(def) = self.registry.get(id) {
                if let Some(rollback_fn) = &def.rollback_fn {
                    if let Err(error) = rollback_fn() {
                        warn!(%error, optimization = %id, "rollback step failed");
                    }
                }
            }
        }

        if let Some(snapshot_id) = &transaction.snapshot_id {
            log.restore_snapshot(snapshot_id)?;
        }

        log.update_status(transaction_id, TransactionStatus::RolledBack)?;
        info!(txn_id = transaction_id, "transaction rolled back");
        Ok(())
    }

    /// Gets the transaction history.
    pub fn history(&self, limit: usize) -> Result<Vec<TransactionRecord>, EngineError> {
        let log = self.transaction_log.lock();
        log.list(limit).map_err(EngineError::Transaction)
    }

    /// Runs preflight checks.
    fn preflight(&self) -> Result<wexo_safety::PreflightReport, EngineError> {
        let is_admin = wexo_sysinfo::is_elevated();
        let free_disk_bytes = get_free_disk_space();
        let pending_reboot = check_pending_reboot();

        Ok(wexo_safety::PreflightReport {
            is_admin,
            free_disk_bytes,
            pending_reboot,
        })
    }

    /// Returns the registry for inspection.
    pub fn registry(&self) -> &OptimizationRegistry {
        &self.registry
    }
}

fn combine_snapshots(specs: &[SnapshotSpec]) -> SnapshotSpec {
    let mut combined = SnapshotSpec::default();
    for spec in specs {
        combined.registry_keys.extend(spec.registry_keys.clone());
        combined.services.extend(spec.services.clone());
        combined.files.extend(spec.files.clone());
    }
    // Deduplicate
    combined.registry_keys.sort();
    combined.registry_keys.dedup();
    combined.services.sort();
    combined.services.dedup();
    combined.files.sort();
    combined.files.dedup();
    combined
}

#[cfg(windows)]
fn get_free_disk_space() -> u64 {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let path: Vec<u16> = OsStr::new("C:\\").encode_wide().chain(Some(0)).collect();
    let mut free_bytes = 0u64;
    let mut total_bytes = 0u64;
    let mut total_free = 0u64;

    unsafe {
        if GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut free_bytes as *mut _ as _,
            &mut total_bytes as *mut _ as _,
            &mut total_free as *mut _ as _,
        ) != 0
        {
            return free_bytes;
        }
    }
    u64::MAX
}

#[cfg(unix)]
fn get_free_disk_space() -> u64 {
    use std::ffi::CString;

    let path = match CString::new("/") {
        Ok(path) => path,
        Err(_) => return u64::MAX,
    };
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `path` is NUL-terminated and `stats` points to writable memory.
    if unsafe { libc::statvfs(path.as_ptr(), &mut stats) } == 0 {
        return (stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64);
    }
    u64::MAX
}

#[cfg(windows)]
fn check_pending_reboot() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Control\\Session Manager", KEY_READ) {
        if let Ok(_) = key.get_value::<String, _>("PendingFileRenameOperations") {
            return true;
        }
    }
    if let Ok(key) = hklm.open_subkey_with_flags("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WindowsUpdate\\Auto Update\\RebootRequired", KEY_READ) {
        if let Ok(_) = key.get_value::<String, _>("") {
            return true;
        }
    }
    false
}

#[cfg(unix)]
fn check_pending_reboot() -> bool {
    // Debian/Ubuntu write these markers; other distributions may not, in which
    // case a reboot is simply not detected.
    ["/var/run/reboot-required", "/run/reboot-required"]
        .iter()
        .any(|path| std::path::Path::new(path).exists())
}