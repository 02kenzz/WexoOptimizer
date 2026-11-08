//! Safety layer: preflight checks, snapshots and the transaction log.

pub mod snapshot;
pub mod transaction;

pub use snapshot::{capture_snapshot, load_snapshot, list_snapshots, restore_snapshot, Snapshot, SnapshotError};
pub use transaction::{TransactionError, TransactionLog, TransactionRecord, TransactionStatus};

use wexo_core::AppSettings;

pub const MIN_FREE_DISK_BYTES: u64 = 500 * 1024 * 1024;

/// Facts gathered before any optimization is allowed to run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreflightReport {
    pub is_admin: bool,
    pub free_disk_bytes: u64,
    pub pending_reboot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreflightBlocker {
    NotElevated,
    LowDiskSpace,
    PendingReboot,
}

impl PreflightReport {
    pub fn blockers(&self) -> Vec<PreflightBlocker> {
        let mut blockers = Vec::new();
        if !self.is_admin {
            blockers.push(PreflightBlocker::NotElevated);
        }
        if self.free_disk_bytes < MIN_FREE_DISK_BYTES {
            blockers.push(PreflightBlocker::LowDiskSpace);
        }
        if self.pending_reboot {
            blockers.push(PreflightBlocker::PendingReboot);
        }
        blockers
    }

    pub fn can_apply(&self) -> bool {
        self.blockers().is_empty()
    }
}

/// Creates the safety layer components (transaction log, snapshot dir) from settings.
pub fn init_safety(settings: &AppSettings) -> Result<(TransactionLog, std::path::PathBuf), transaction::TransactionError> {
    let data_dir = if let Some(custom) = &settings.data_dir {
        std::path::PathBuf::from(custom)
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("WexoOptimizer")
    };
    let log = TransactionLog::new(&data_dir)?;
    Ok((log, data_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unelevated_process_cannot_apply() {
        let report = PreflightReport {
            is_admin: false,
            free_disk_bytes: u64::MAX,
            pending_reboot: false,
        };
        assert_eq!(report.blockers(), vec![PreflightBlocker::NotElevated]);
        assert!(!report.can_apply());
    }

    #[test]
    fn healthy_environment_can_apply() {
        let report = PreflightReport {
            is_admin: true,
            free_disk_bytes: MIN_FREE_DISK_BYTES,
            pending_reboot: false,
        };
        assert!(report.can_apply());
    }
}