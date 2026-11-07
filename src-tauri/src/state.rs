use std::sync::{Mutex, RwLock};

use tracing_appender::non_blocking::WorkerGuard;
use wexo_core::AppSettings;
use wexo_sysinfo::LiveCollector;

use crate::config::AppPaths;

/// Shared state managed by Tauri and injected into commands.
pub struct AppState {
    pub paths: AppPaths,
    pub settings: RwLock<AppSettings>,
    pub collector: LiveCollector,
    // Kept alive for the whole process so buffered log lines are flushed on exit.
    _log_guard: Mutex<Option<WorkerGuard>>,
}

impl AppState {
    pub fn new(paths: AppPaths, settings: AppSettings, log_guard: Option<WorkerGuard>) -> Self {
        Self {
            paths,
            settings: RwLock::new(settings),
            collector: LiveCollector::new(),
            _log_guard: Mutex::new(log_guard),
        }
    }

    pub fn settings_snapshot(&self) -> AppSettings {
        match self.settings.read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}
