use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tracing_appender::non_blocking::WorkerGuard;
use wexo_core::{AppSettings, LiveMetrics, OverlayStats};
use wexo_monitor::{MetricSource, MonitorHandle, DEFAULT_HISTORY_CAPACITY};
use wexo_optimizer::OptimizationEngine;
use wexo_safety::init_safety;
use wexo_sysinfo::{LiveCollector, LiveSampler, OverlayProbe};
use wexo_tools::JobManager;

use crate::config::AppPaths;
use crate::events;

/// Adapts the sysinfo/PDH sampler to the monitor's `MetricSource` trait without
/// coupling `wexo-monitor` to Windows.
struct SamplerSource(LiveSampler);

impl MetricSource for SamplerSource {
    fn sample(&mut self) -> LiveMetrics {
        self.0.sample()
    }
}

/// Shared state managed by Tauri and injected into commands.
pub struct AppState {
    pub paths: AppPaths,
    pub settings: RwLock<AppSettings>,
    pub collector: LiveCollector,
    /// Running monitor thread. `None` until the Tauri setup hook starts it,
    /// and again after shutdown.
    monitor: Mutex<Option<MonitorHandle>>,
    /// Optimization engine with safety layer.
    pub engine: Arc<Mutex<OptimizationEngine>>,
    /// GPU counters and ETW frame timing for the overlay. Opened lazily, so a
    /// disabled overlay costs nothing.
    overlay: Mutex<OverlayProbe>,
    /// Cleanup and long-running maintenance commands from the Tools page.
    pub tools: JobManager,
    // Kept alive for the whole process so buffered log lines are flushed on exit.
    _log_guard: Mutex<Option<WorkerGuard>>,
}

impl AppState {
    pub fn new(paths: AppPaths, settings: AppSettings, log_guard: Option<WorkerGuard>) -> Self {
        let (transaction_log, _data_dir) =
            init_safety(&settings).expect("failed to initialize safety layer");
        let engine = OptimizationEngine::new(transaction_log);

        Self {
            paths,
            settings: RwLock::new(settings),
            collector: LiveCollector::new(),
            monitor: Mutex::new(None),
            engine: Arc::new(Mutex::new(engine)),
            overlay: Mutex::new(OverlayProbe::new()),
            tools: JobManager::new(),
            _log_guard: Mutex::new(log_guard),
        }
    }

    pub fn settings_snapshot(&self) -> AppSettings {
        match self.settings.read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Starts the background monitor if it is not already running.
    ///
    /// Called from the Tauri setup hook, where an `AppHandle` is available for
    /// forwarding samples to the webview.
    pub fn start_monitor(&self, app: AppHandle) {
        let mut guard = lock(&self.monitor);
        if guard.is_some() {
            return;
        }

        let interval =
            Duration::from_millis(u64::from(self.settings_snapshot().monitor_interval_ms));
        let sink = move |sample: &LiveMetrics| {
            if let Err(error) = app.emit(events::MONITOR_METRICS, sample) {
                tracing::debug!(%error, "no se pudo emitir la muestra del monitor");
            }
        };

        *guard = Some(MonitorHandle::start(
            SamplerSource(LiveSampler::new()),
            sink,
            interval,
            DEFAULT_HISTORY_CAPACITY,
        ));
        tracing::info!(interval_ms = interval.as_millis(), "monitor started");
    }

    /// Stops and joins the monitor thread. Safe to call more than once.
    pub fn stop_monitor(&self) {
        let handle = lock(&self.monitor).take();
        if let Some(mut handle) = handle {
            handle.stop();
            tracing::info!("monitor stopped");
        }
    }

    /// Applies a new sampling interval without losing history.
    pub fn set_monitor_interval(&self, interval: Duration) {
        if let Some(handle) = lock(&self.monitor).as_ref() {
            if !handle.set_interval(interval) {
                tracing::warn!("el monitor no está en ejecución; intervalo no aplicado");
            }
        }
    }

    pub fn monitor_interval_ms(&self) -> Option<u64> {
        lock(&self.monitor).as_ref().map(MonitorHandle::interval_ms)
    }

    /// Copy of the monitor history, oldest first. Empty before the first tick.
    pub fn metrics_history(&self) -> Vec<LiveMetrics> {
        lock(&self.monitor)
            .as_ref()
            .map(MonitorHandle::history)
            .unwrap_or_default()
    }

    pub fn latest_metrics(&self) -> Option<LiveMetrics> {
        lock(&self.monitor).as_ref().and_then(MonitorHandle::latest)
    }

    /// Opens the overlay probes on first use (GPU counters, ETW timing).
    pub fn start_overlay(&self) {
        lock(&self.overlay).ensure_running();
    }

    /// Snapshot rendered by the overlay window. Before the first monitor sample
    /// the metrics are empty rather than invented.
    pub fn overlay_stats(&self) -> OverlayStats {
        let metrics = self
            .latest_metrics()
            .unwrap_or_else(|| LiveMetrics::empty(1, 0));
        lock(&self.overlay).sample(&metrics)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
