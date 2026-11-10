mod commands;
mod config;
mod elevation;
mod error;
mod events;
mod logging;
mod overlay;
mod state;

use state::AppState;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    LogicalSize, Manager, Runtime, Size, WebviewWindow, WindowEvent,
};

/// Preferred and minimum main-window size, in logical pixels.
const IDEAL_WIDTH: f64 = 1320.0;
const IDEAL_HEIGHT: f64 = 860.0;
const MIN_WIDTH: f64 = 900.0;
const MIN_HEIGHT: f64 = 620.0;

/// Boots the Tauri application.
pub fn run() {
    // The app always opens with administrator rights: an unelevated launch
    // starts an elevated copy and lets it do the work.
    if elevation::should_hand_off() {
        return;
    }

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

            // Reopen the performance overlay when it was left enabled.
            let state = app.state::<AppState>();
            if state.settings_snapshot().overlay_enabled {
                state.start_overlay();
                overlay::apply(app.handle(), true);
            }

            // Handle window close event (minimize to tray if enabled)
            if let Some(window) = app.get_webview_window("main") {
                fit_main_window(&window);
                let _ = window.show();
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
            commands::get_overlay_stats,
            commands::run_diagnostics,
            commands::get_settings,
            commands::save_settings,
            commands::show_window,
            commands::hide_window,
            commands::get_optimizations,
            commands::apply_optimizations,
            commands::get_transaction_history,
            commands::rollback_transaction,
            commands::scan_cleanup,
            commands::run_cleanup,
            commands::start_tool_job,
            commands::get_tool_job,
            commands::get_tool_jobs,
            commands::cancel_tool_job,
            commands::maintain_database,
            commands::open_logs_dir,
            commands::relaunch_as_admin,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Wexo Optimizer");

    app.run(|handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            handle.state::<AppState>().stop_monitor();
        }
    });
}

/// Fits the main window to the monitor it opens on, leaving room for the
/// taskbar and the window frame, then centers it. It never maximizes.
fn fit_main_window<R: Runtime>(window: &WebviewWindow<R>) {
    let monitor = match window.current_monitor() {
        Ok(Some(monitor)) => monitor,
        _ => return,
    };

    let screen = monitor.size().to_logical::<f64>(monitor.scale_factor());
    let available_width = (screen.width - 96.0).max(MIN_WIDTH);
    let available_height = (screen.height - 128.0).max(MIN_HEIGHT);
    let width = IDEAL_WIDTH.min(available_width);
    let height = IDEAL_HEIGHT.min(available_height);

    tracing::info!(
        width,
        height,
        scale = monitor.scale_factor(),
        "main window fitted to display"
    );
    let _ = window.set_size(Size::Logical(LogicalSize::new(width, height)));
    let _ = window.center();
}

/// Creates the system tray with Show/Quit menu.
///
/// The tray is built here instead of in `tauri.conf.json`: declaring it in both
/// places makes Tauri create two icons with the same id.
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
        // Left click opens the window; the menu stays on right click.
        .show_menu_on_left_click(false)
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
