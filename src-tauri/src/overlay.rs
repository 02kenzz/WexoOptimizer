//! Always-on-top, click-through performance overlay window.
//!
//! The window is created on demand (only when the setting is on) and reused
//! afterwards. It is transparent, undecorated, skipped from the taskbar and
//! ignores the cursor, so it never steals a click from the game underneath.

use tauri::{
    LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use wexo_core::{OverlayOrientation, OverlayPosition, OverlaySettings};

use crate::state::AppState;

/// Label of the overlay webview, matched by the frontend to pick its layout.
pub const LABEL: &str = "overlay";

/// Frame around the card, in logical pixels. The card is anchored to the
/// matching corner of that frame, so the transparent remainder of the window is
/// never visible. A horizontal panel needs a wider, shorter frame.
const VERTICAL_SIZE: (f64, f64) = (308.0, 328.0);
const HORIZONTAL_SIZE: (f64, f64) = (600.0, 172.0);
/// Small gap to the screen edges: the panel sits high without touching them.
const MARGIN: f64 = 8.0;

fn frame_size(overlay: &OverlaySettings) -> (f64, f64) {
    match overlay.orientation {
        OverlayOrientation::Vertical => VERTICAL_SIZE,
        OverlayOrientation::Horizontal => HORIZONTAL_SIZE,
    }
}

/// Shows or hides the overlay according to `enabled`.
///
/// Creating the window always runs on its own thread: doing it inside a
/// synchronous command or an event handler blocks the WebView2 message pump on
/// Windows and surfaces as `0x8007139F` (deadlock).
pub fn apply(app: &tauri::AppHandle, enabled: bool) {
    if !enabled {
        hide(app);
        return;
    }

    // An existing window only needs repositioning, which is safe on the
    // caller's thread and keeps settings changes instant.
    if app.get_webview_window(LABEL).is_some() {
        show(app);
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
    let overlay = overlay_settings(app);

    if let Some(window) = app.get_webview_window(LABEL) {
        place(&window, &overlay);
        let _ = window.show();
        return;
    }

    match build(app, true) {
        Ok(window) => {
            place(&window, &overlay);
            let _ = window.show();
        }
        Err(error) => {
            // An opaque panel is still better than no overlay at all.
            tracing::warn!(%error, "overlay transparente no disponible, se reintenta opaco");
            match build(app, false) {
                Ok(window) => {
                    place(&window, &overlay);
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

/// Current overlay preferences, read from the state the commands write to.
fn overlay_settings(app: &tauri::AppHandle) -> OverlaySettings {
    app.state::<AppState>().settings_snapshot().overlay
}

fn build(app: &tauri::AppHandle, transparent: bool) -> tauri::Result<WebviewWindow> {
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Wexo Overlay")
        .inner_size(VERTICAL_SIZE.0, VERTICAL_SIZE.1)
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
    Ok(window)
}

/// Anchors the window to the corner picked in settings and keeps its frame in
/// sync with the chosen orientation.
fn place(window: &WebviewWindow, overlay: &OverlaySettings) {
    let Ok(Some(monitor)) = window.primary_monitor() else {
        return;
    };
    let scale = monitor.scale_factor();
    let size = monitor.size();
    let origin = monitor.position();

    let (logical_width, logical_height) = frame_size(overlay);
    let margin = (MARGIN * scale).round() as i32;
    let width = (logical_width * scale).round() as i32;
    let height = (logical_height * scale).round() as i32;

    let _ = window.set_size(LogicalSize::new(logical_width, logical_height));

    let x = match overlay.position {
        OverlayPosition::TopLeft | OverlayPosition::BottomLeft => origin.x + margin,
        OverlayPosition::TopRight | OverlayPosition::BottomRight => {
            origin.x + size.width as i32 - width - margin
        }
    };
    let y = match overlay.position {
        OverlayPosition::TopLeft | OverlayPosition::TopRight => origin.y + margin,
        OverlayPosition::BottomLeft | OverlayPosition::BottomRight => {
            origin.y + size.height as i32 - height - margin
        }
    };
    let _ = window.set_position(PhysicalPosition::new(x, y));
}
