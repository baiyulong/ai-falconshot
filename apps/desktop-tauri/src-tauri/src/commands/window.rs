use floating_core::{FloatingState, FloatingWindow, TransformState};
use platform_windows::floating::WindowsFloatingWindow;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, WebviewWindow};
/// Live pinned images. Windows are created on the app's main thread (so the
/// Tauri event loop pumps their messages) and kept here for the app lifetime.
static PINS: OnceLock<Mutex<Vec<WindowsFloatingWindow>>> = OnceLock::new();

fn pins() -> &'static Mutex<Vec<WindowsFloatingWindow>> {
    PINS.get_or_init(|| Mutex::new(Vec::new()))
}

fn next_pin_id() -> String {
    format!(
        "pin-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    )
}

#[tauri::command]
pub fn start_window_drag(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

/// Close the editor window. Focus returns to whatever window was behind it;
/// the main window stays where the user left it (shown only from the tray).
#[tauri::command]
pub fn close_editor(window: WebviewWindow) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

/// Pin the edited capture as an always-on-top draggable floating image.
/// (x, y, width, height) is the image rect in physical screen coordinates.
#[tauri::command]
pub async fn pin_image(
    app: AppHandle,
    path: String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    app.run_on_main_thread(move || {
        let mut pin = WindowsFloatingWindow::new();
        let state = FloatingState {
            id: next_pin_id(),
            image_path: String::new(),
            x,
            y,
            width: width.max(1) as u32,
            height: height.max(1) as u32,
            transform: TransformState::default(),
            opacity: 1.0,
            always_on_top: true,
            mouse_passthrough: false,
            locked_position: false,
            locked_size: false,
            group_id: None,
        };
        match pin.create(std::path::Path::new(&path), &state) {
            Ok(()) => {
                pins().lock().unwrap().push(pin);
            }
            Err(e) => eprintln!("pin_image failed: {e}"),
        }
    })
    .map_err(|e| e.to_string())
}
