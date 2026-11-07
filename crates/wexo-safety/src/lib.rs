//! Safety layer: preflight checks, snapshots and the transaction log.
//!
//! M0 ships the pure preflight decision logic; snapshot and SQLite storage land in M2.

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
