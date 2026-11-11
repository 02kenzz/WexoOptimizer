use std::time::Duration;

use tauri::{Manager, State};
use wexo_core::{
    AppInfo, AppSettings, DiagnosticReport, LiveMetrics, Optimization, OverlayStats, SystemProfile,
    VerificationResult,
};
use wexo_sysinfo::SystemCollector;
use wexo_tools::{CleanKind, CleanReport, JobKind, JobSnapshot};

use crate::config;
use crate::error::CommandError;
use crate::state::AppState;

#[tauri::command]
pub fn ping() -> &'static str {
    "pong"
}

#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        name: "Wexo Optimizer".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: format!("{} {}", wexo_sysinfo::os_name(), wexo_sysinfo::os_version()),
        arch: std::env::consts::ARCH.to_string(),
        is_admin: wexo_sysinfo::is_elevated(),
        log_dir: state.paths.logs_dir.to_string_lossy().to_string(),
    }
}

#[tauri::command]
pub fn get_system_profile(state: State<'_, AppState>) -> SystemProfile {
    state.collector.profile()
}

/// Re-collects static hardware data (WMI) and returns the refreshed profile.
#[tauri::command]
pub fn refresh_profile(state: State<'_, AppState>) -> SystemProfile {
    state.collector.refresh_hardware();
    state.collector.profile()
}

/// Most recent monitor sample, or `null` before the first tick.
#[tauri::command]
pub fn get_live_metrics(state: State<'_, AppState>) -> Option<LiveMetrics> {
    state.latest_metrics()
}

/// Full in-memory history, oldest first. Used to backfill charts on mount.
#[tauri::command]
pub fn get_metrics_history(state: State<'_, AppState>) -> Vec<LiveMetrics> {
    state.metrics_history()
}

/// Snapshot rendered by the performance overlay window.
#[tauri::command]
pub fn get_overlay_stats(state: State<'_, AppState>) -> OverlayStats {
    state.overlay_stats()
}

/// Current sampling interval, in milliseconds.
#[tauri::command]
pub fn get_monitor_interval(state: State<'_, AppState>) -> Option<u64> {
    state.monitor_interval_ms()
}

/// Runs the diagnostic engine over the current profile and monitor window.
#[tauri::command]
pub fn run_diagnostics(state: State<'_, AppState>) -> DiagnosticReport {
    let profile = state.collector.profile();
    let samples = state.metrics_history();
    wexo_core::analyze(&profile, &samples)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings_snapshot()
}

#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, CommandError> {
    let previous = state.settings_snapshot();
    let settings = settings.sanitize();
    config::save_settings(&state.paths, &settings)?;

    match state.settings.write() {
        Ok(mut guard) => *guard = settings.clone(),
        Err(poisoned) => *poisoned.into_inner() = settings.clone(),
    }

    state.set_monitor_interval(Duration::from_millis(u64::from(
        settings.monitor_interval_ms,
    )));

    if settings.overlay_enabled {
        state.start_overlay();
    }

    // A setting save must not pop the overlay open: only an explicit off→on
    // transition shows it, and off hides it. Any other save just repositions
    // the window when it already exists.
    match (previous.overlay_enabled, settings.overlay_enabled) {
        (false, true) => crate::overlay::apply(&app, true),
        (true, false) => crate::overlay::hide(&app),
        (true, true) => crate::overlay::refresh(&app),
        (false, false) => {}
    }

    tracing::info!(?settings, "settings updated");
    Ok(settings)
}

/// Shows the main window (e.g., from tray click).
#[tauri::command]
pub fn show_window(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .map(|w| {
            let _ = w.show();
            let _ = w.set_focus();
        })
        .ok_or_else(|| "main window not found".to_string())
}

/// Hides the main window to tray.
#[tauri::command]
pub fn hide_window(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;
    window.hide().map_err(|e| e.to_string())
}

/// Returns all known optimizations with their availability evaluated.
#[tauri::command]
pub fn get_optimizations(state: State<'_, AppState>) -> Vec<Optimization> {
    let engine = state.engine.lock().unwrap();
    let mut items: Vec<Optimization> = engine
        .registry()
        .iter()
        .map(|def| def.meta.clone())
        .collect();
    items.sort_by(|a, b| a.title.cmp(&b.title));
    items
}

/// Opens the log directory in the platform file manager.
#[tauri::command]
pub fn open_logs_dir(state: State<'_, AppState>) -> Result<(), String> {
    let dir = &state.paths.logs_dir;

    #[cfg(windows)]
    let opened = std::process::Command::new("explorer.exe").arg(dir).spawn();
    #[cfg(target_os = "linux")]
    let opened = std::process::Command::new("xdg-open").arg(dir).spawn();
    #[cfg(not(any(windows, target_os = "linux")))]
    let opened = std::process::Command::new("open").arg(dir).spawn();

    opened.map(|_| ()).map_err(|error| error.to_string())
}

/// Restarts the application elevated so admin-only tweaks can run.
///
/// The app already elevates itself on startup; this is the retry path for a
/// user who declined the UAC/polkit prompt.
#[tauri::command]
pub fn relaunch_as_admin(app: tauri::AppHandle) -> Result<(), String> {
    if crate::elevation::already_elevated() {
        return Err("la aplicación ya se está ejecutando como administrador".to_string());
    }
    if !crate::elevation::relaunch_elevated() {
        return Err("no se pudo solicitar la elevación".to_string());
    }
    app.exit(0);
    Ok(())
}

/// Applies selected optimizations atomically with safety checks.
#[tauri::command]
pub fn apply_optimizations(
    state: State<'_, AppState>,
    optimization_ids: Vec<String>,
    dry_run: bool,
) -> Result<ApplyResultResponse, String> {
    let engine = state.engine.lock().unwrap();
    let result = engine
        .apply(&optimization_ids, dry_run)
        .map_err(|e| e.to_string())?;

    let results: Vec<OptimizationResultResponse> = result
        .results
        .into_iter()
        .map(|r| OptimizationResultResponse {
            optimization_id: r.optimization_id,
            applied: r.applied,
            verification: r.verification,
            message: r.message,
            benchmark_delta: r.benchmark_delta,
        })
        .collect();

    Ok(ApplyResultResponse {
        transaction_id: result.transaction.id,
        results,
    })
}

/// Returns transaction history.
#[tauri::command]
pub fn get_transaction_history(
    state: State<'_, AppState>,
    limit: usize,
) -> Vec<TransactionRecordResponse> {
    let engine = state.engine.lock().unwrap();
    engine
        .history(limit)
        .unwrap_or_default()
        .into_iter()
        .map(|t| TransactionRecordResponse {
            id: t.id,
            timestamp_ms: t.timestamp_ms,
            optimization_ids: t.optimization_ids,
            status: t.status,
            rollback_available: t.rollback_available,
            snapshot_id: t.snapshot_id,
            message: t.message,
        })
        .collect()
}

/// Rolls back a specific transaction.
#[tauri::command]
pub fn rollback_transaction(
    state: State<'_, AppState>,
    transaction_id: String,
) -> Result<(), String> {
    let engine = state.engine.lock().unwrap();
    engine
        .rollback_transaction(&transaction_id)
        .map_err(|e| e.to_string())
}

/// Measures what a cleanup would free. Deletes nothing.
#[tauri::command(async)]
pub fn scan_cleanup(kind: String) -> Result<CleanReport, String> {
    let kind = clean_kind(&kind)?;
    Ok(wexo_tools::scan(kind))
}

/// Deletes the files of one cleanup kind and reports what was actually freed.
#[tauri::command(async)]
pub fn run_cleanup(kind: String) -> Result<CleanReport, String> {
    let kind = clean_kind(&kind)?;
    Ok(wexo_tools::clean(kind))
}

fn clean_kind(value: &str) -> Result<CleanKind, String> {
    CleanKind::parse(value).ok_or_else(|| format!("tipo de limpieza desconocido: {value}"))
}

/// Starts a long-running maintenance command (`sfc`, `dism`, `defrag`, `trim`,
/// `check-disk`). Returns the initial snapshot; the UI polls `get_tool_job`.
#[tauri::command]
pub fn start_tool_job(state: State<'_, AppState>, tool: String) -> Result<JobSnapshot, String> {
    let kind = JobKind::parse(&tool).ok_or_else(|| format!("herramienta desconocida: {tool}"))?;
    state.tools.start(kind)
}

/// Current state of one job, or `null` when the id is unknown.
#[tauri::command]
pub fn get_tool_job(state: State<'_, AppState>, id: String) -> Option<JobSnapshot> {
    state.tools.get(&id)
}

/// Every remembered job, oldest first. Used to restore the console on mount.
#[tauri::command]
pub fn get_tool_jobs(state: State<'_, AppState>) -> Vec<JobSnapshot> {
    state.tools.all()
}

/// Asks a running job to stop. Returns `false` when it was not running.
#[tauri::command]
pub fn cancel_tool_job(state: State<'_, AppState>, id: String) -> bool {
    state.tools.cancel(&id)
}

/// Compacts (`vacuum`) or reindexes the transaction database.
#[tauri::command(async)]
pub fn maintain_database(
    state: State<'_, AppState>,
    action: String,
) -> Result<wexo_safety::DatabaseReport, String> {
    let engine = state.engine.lock().unwrap();
    let result = match action.as_str() {
        "vacuum" => engine.vacuum_database(),
        "reindex" => engine.reindex_database(),
        other => return Err(format!("acción desconocida: {other}")),
    };
    result.map_err(|error| error.to_string())
}

/// Checks every installed GPU against the vendor's latest driver.
///
/// Network work only happens for NVIDIA cards; AMD/Intel report `unknown`
/// instead of a guessed version. Runs off the async runtime because the HTTP
/// lookup blocks.
#[tauri::command]
pub async fn check_gpu_drivers(
    state: State<'_, AppState>,
) -> Result<Vec<wexo_sysinfo::drivers::GpuDriverInfo>, String> {
    let gpus = state.collector.profile().gpus;
    tauri::async_runtime::spawn_blocking(move || wexo_sysinfo::drivers::check_all(&gpus))
        .await
        .map_err(|error| error.to_string())
}

/// Opens the vendor's update software, falling back to its official page.
#[tauri::command(async)]
pub fn open_gpu_driver_update(vendor: String, url: Option<String>) -> Result<String, String> {
    wexo_sysinfo::drivers::open_update(&vendor, url.as_deref())
}

/// Response types for Tauri commands (serializable).
#[derive(serde::Serialize)]
pub struct ApplyResultResponse {
    pub transaction_id: String,
    pub results: Vec<OptimizationResultResponse>,
}

#[derive(serde::Serialize)]
pub struct OptimizationResultResponse {
    pub optimization_id: String,
    pub applied: bool,
    pub verification: VerificationResult,
    pub message: Option<String>,
    pub benchmark_delta: Option<wexo_core::BenchmarkDelta>,
}

#[derive(serde::Serialize)]
pub struct TransactionRecordResponse {
    pub id: String,
    pub timestamp_ms: u64,
    pub optimization_ids: Vec<String>,
    pub status: wexo_safety::TransactionStatus,
    pub rollback_available: bool,
    pub snapshot_id: Option<String>,
    pub message: Option<String>,
}
