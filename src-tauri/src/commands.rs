use tauri::State;
use wexo_core::{AppInfo, AppSettings, SystemProfile};
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

    tracing::info!(?settings, "settings updated");
    Ok(settings)
}
