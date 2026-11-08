mod commands;
mod config;
mod error;
mod events;
mod logging;
mod state;

use state::AppState;
use tauri::{
    Manager, Runtime,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    WindowEvent,
};

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
            // Start monitor
            app.state::<AppState>().start_monitor(app.handle().clone());

            // Create system tray
            create_tray(app.handle().clone())?;

            // Handle window close event (minimize to tray if enabled)
            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        let settings = handle.state::<AppState>().settings_snapshot();
                        if settings.minimize_to_tray {
                            api.prevent_close();
                            let _ = handle.get_webview_window("main").map(|w| w.hide());
                        }
                    }
                });
            }

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
            commands::show_window,
            commands::hide_window,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Wexo Optimizer");

    app.run(|handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            handle.state::<AppState>().stop_monitor();
        }
    });
}

/// Creates the system tray with Show/Quit menu.
fn create_tray<R: Runtime>(handle: tauri::AppHandle<R>) -> tauri::Result<()> {
    let show_item = MenuItemBuilder::new("Mostrar")
        .id("show")
        .accelerator("CmdOrCtrl+Shift+W")
        .build(&handle)?;

    let quit_item = MenuItemBuilder::new("Salir")
        .id("quit")
        .accelerator("CmdOrCtrl+Q")
        .build(&handle)?;

    let menu = MenuBuilder::new(&handle)
        .items(&[&show_item, &quit_item])
        .build()?;

    let _tray = TrayIconBuilder::new()
        .icon(handle.default_window_icon().cloned().unwrap())
        .icon_as_template(true)
        .menu(&menu)
        .tooltip("Wexo Optimizer")
        .on_menu_event(move |handle, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                // Force quit bypassing minimize-to-tray
                handle.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(&handle)?;

    Ok(())
}
