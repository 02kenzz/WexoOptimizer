//! Non-Windows stand-in for the WMI collectors.
//!
//! Linux has no WMI: static hardware details come from `sysinfo` and `/sys`,
//! so the cache stays empty and [`enrich_profile`] is a no-op. The API mirrors
//! the Windows module so the rest of the crate is platform-independent.

use std::sync::Arc;

use wexo_core::{DiskKind, SystemProfile};

#[derive(Debug, Clone, Default)]
pub struct WmiSnapshot {
    pub collected_at_ms: u64,
    pub disks: Vec<WmiDisk>,
}

#[derive(Debug, Clone)]
pub struct WmiDisk {
    pub model: String,
    pub size_bytes: u64,
    pub kind: DiskKind,
    pub bus_type: String,
}

/// Empty cache: there is nothing to collect without WMI.
#[derive(Debug, Default)]
pub struct WmiCache;

impl WmiCache {
    pub fn new() -> Self {
        Self
    }

    pub fn current(self: &Arc<Self>) -> Option<Arc<WmiSnapshot>> {
        None
    }

    pub fn refresh_async(self: &Arc<Self>) {}

    pub fn refresh_now(&self) -> Option<Arc<WmiSnapshot>> {
        None
    }
}

/// No-op: `sysinfo` already filled everything this platform can report.
pub fn enrich_profile(_profile: &mut SystemProfile, _snapshot: &WmiSnapshot) {}
