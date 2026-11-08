use std::time::Duration;

use tauri::{Manager, State};
use wexo_core::{AppInfo, AppSettings, DiagnosticReport, LiveMetrics, SystemProfile};
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
