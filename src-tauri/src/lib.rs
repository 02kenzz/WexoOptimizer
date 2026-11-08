mod commands;
mod config;
mod error;
mod events;
mod logging;
mod state;

use state::AppState;
use tauri::Manager;

/// Boots the Tauri application.
pub fn run() {
    let paths = config::AppPaths::discover();
    if let Err(error) = paths.ensure_dirs() {
        eprintln!("could not create application directories: {error}");
    }

    let log_guard = logging::init(&paths);
    let settings = config::load_settings(&paths);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        language = ?settings.language,
        "Wexo Optimizer starting"
    );

    let state = AppState::new(paths, settings, log_guard);

    let app = tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            app.state::<AppState>().start_monitor(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::get_app_info,
            commands::get_system_profile,
            commands::refresh_profile,
            commands::get_live_metrics,
            commands::get_metrics_history,
            commands::get_monitor_interval,
            commands::run_diagnostics,
            commands::get_settings,
            commands::save_settings,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Wexo Optimizer");

    app.run(|handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            handle.state::<AppState>().stop_monitor();
        }
    });
}
