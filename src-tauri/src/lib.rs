mod commands;
mod config;
mod error;
mod events;
mod logging;
mod state;

use state::AppState;

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

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::get_app_info,
            commands::get_system_profile,
            commands::get_settings,
            commands::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Wexo Optimizer");
}
