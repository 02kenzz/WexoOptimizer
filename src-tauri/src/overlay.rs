//! Always-on-top, click-through performance overlay window.
//!
//! The window is created on demand (only when the setting is on) and reused
//! afterwards. It is transparent, undecorated, skipped from the taskbar and
//! ignores the cursor, so it never steals a click from the game underneath.

use tauri::{Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Label of the overlay webview, matched by the frontend to pick its layout.
pub const LABEL: &str = "overlay";

/// Horizontal bar: four label/value pairs in a single row.
const WIDTH: f64 = 380.0;
const HEIGHT: f64 = 38.0;
const MARGIN: f64 = 16.0;

/// Shows or hides the overlay according to `enabled`.
///
/// Window creation always runs on its own thread: doing it inside a synchronous
/// command or an event handler blocks the WebView2 message pump on Windows and
/// surfaces as `0x8007139F` ("class not registered"/deadlock).
pub fn apply(app: &tauri::AppHandle, enabled: bool) {
    if !enabled {
        hide(app);
        return;
    }

    let handle = app.clone();
    let spawned = std::thread::Builder::new()
        .name("wexo-overlay".to_string())
        .spawn(move || show(&handle));
    if let Err(error) = spawned {
        tracing::warn!(%error, "no se pudo lanzar el hilo del overlay");
        show(app);
    }
}

/// Shows the overlay, creating the window the first time.
pub fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.show();
        return;
    }

    match build(app, true) {
        Ok(window) => {
            let _ = window.show();
        }
        Err(error) => {
            // An opaque panel is still better than no overlay at all.
            tracing::warn!(%error, "overlay transparente no disponible, se reintenta opaco");
            match build(app, false) {
                Ok(window) => {
                    let _ = window.show();
                }
                Err(error) => tracing::warn!(%error, "no se pudo crear el overlay"),
            }
        }
    }
}

/// Hides the overlay without destroying it, so re-enabling is instant.
pub fn hide(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}

fn build(app: &tauri::AppHandle, transparent: bool) -> tauri::Result<WebviewWindow> {
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Wexo Overlay")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(transparent)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .focused(false)
        .visible(false)
        .build()?;

    // Click-through: inputs go to whatever is underneath.
    let _ = window.set_ignore_cursor_events(true);
    place_top_right(&window);
    Ok(window)
}

/// Anchors the overlay to the top-right corner of the primary monitor.
fn place_top_right(window: &WebviewWindow) {
    let Ok(Some(monitor)) = window.primary_monitor() else {
        return;
    };
    let scale = monitor.scale_factor();
    let size = monitor.size();
    let origin = monitor.position();

    let margin = (MARGIN * scale).round() as i32;
    let width = (WIDTH * scale).round() as i32;
    // Keep the bar close to the corner, like a game HUD.
    let top = (MARGIN * scale).round() as i32;

    let x = origin.x + size.width as i32 - width - margin;
    let y = origin.y + top;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}
