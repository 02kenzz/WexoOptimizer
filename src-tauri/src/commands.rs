use std::time::Duration;

use tauri::{Manager, State};
use wexo_core::{AppInfo, AppSettings, DiagnosticReport, LiveMetrics, Optimization, SystemProfile, VerificationResult};
use wexo_sysinfo::SystemCollector;

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
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, CommandError> {
    let settings = settings.sanitize();
    config::save_settings(&state.paths, &settings)?;

    match state.settings.write() {
        Ok(mut guard) => *guard = settings.clone(),
        Err(poisoned) => *poisoned.into_inner() = settings.clone(),
    }

    state.set_monitor_interval(Duration::from_millis(u64::from(
        settings.monitor_interval_ms,
    )));

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
    let window = app.get_webview_window("main").ok_or("main window not found")?;
    window.hide().map_err(|e| e.to_string())
}

/// Returns all known optimizations with their availability evaluated.
#[tauri::command]
pub fn get_optimizations(state: State<'_, AppState>) -> Vec<Optimization> {
    let engine = state.engine.lock().unwrap();
    let mut items: Vec<Optimization> = engine.registry().iter().map(|def| def.meta.clone()).collect();
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
    let result = engine.apply(&optimization_ids, dry_run).map_err(|e| e.to_string())?;

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
pub fn get_transaction_history(state: State<'_, AppState>, limit: usize) -> Vec<TransactionRecordResponse> {
    let engine = state.engine.lock().unwrap();
    engine.history(limit)
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
pub fn rollback_transaction(state: State<'_, AppState>, transaction_id: String) -> Result<(), String> {
    let engine = state.engine.lock().unwrap();
    engine.rollback_transaction(&transaction_id).map_err(|e| e.to_string())
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
