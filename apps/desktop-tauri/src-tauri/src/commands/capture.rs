use capture_core::{CaptureBackend, CaptureOptions, Rect};
use platform_windows::capture::WindowsCaptureBackend;
use platform_windows::overlay::{OverlayResult, WindowsOverlay};
use settings_core::SettingsBackend;
use std::sync::atomic::AtomicBool;

/// Guards against stacking overlay sessions when the hotkey is mashed.
pub static CAPTURE_BUSY: AtomicBool = AtomicBool::new(false);

fn hotkeys_paused() -> bool {
    settings_core::JsonSettingsBackend::new(settings_core::JsonSettingsBackend::default_path())
        .load()
        .map(|s| s.hotkeys.paused)
        .unwrap_or(false)
}

/// Run one capture (blocking: the overlay pumps its own message loop) and
/// open the editor over the result. Callable from any thread.
pub fn capture_and_edit(app: &tauri::AppHandle) -> Result<(), String> {
    if hotkeys_paused() {
        return Err("hotkeys paused".to_string());
    }
    let result = {
        let mut overlay = WindowsOverlay::new();
        overlay.show_and_select()
    }
    .map_err(|e| e.to_string())?;

    match result {
        OverlayResult::Selected(rect, image) => {
            let path = save_image(&image)?;
            open_editor_window(app, &path, &rect)
        }
        OverlayResult::Cancelled => Err("cancelled".to_string()),
    }
}

#[tauri::command]
pub async fn start_capture(app: tauri::AppHandle) -> Result<(), String> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || capture_and_edit(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<String, String> {
    let rect = Rect::new(x, y, width, height);
    tokio::task::spawn_blocking(move || capture_rect_to_file(&rect))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn capture_fullscreen(app: tauri::AppHandle) -> Result<(), String> {
    let app = app.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let backend = WindowsCaptureBackend::new().map_err(|e| e.to_string())?;
        let frame = backend.capture_fullscreen().map_err(|e| e.to_string())?;
        let path = save_image(&frame.image)?;
        open_editor_window(&app, &path, &frame.monitor.bounds)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn capture_rect_to_file(rect: &Rect) -> Result<String, String> {
    let backend = WindowsCaptureBackend::new().map_err(|e| e.to_string())?;
    let options = CaptureOptions {
        region: Some(rect.clone()),
        ..Default::default()
    };
    let frame = backend
        .capture_region(&options)
        .map_err(|e| e.to_string())?;
    save_image(&frame.image)
}

fn save_image(img: &image::RgbaImage) -> Result<String, String> {
    let dir = settings_core::JsonSettingsBackend::app_data_dir().join("captures");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = dir.join(format!("capture_{}.png", timestamp));

    img.save(&path).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

/// Create the borderless editor window exactly over the captured area.
/// A fresh window is used for every capture (instead of moving/resizing the
/// main window) so the webview composition always matches the geometry.
fn open_editor_window(app: &tauri::AppHandle, path: &str, rect: &Rect) -> Result<(), String> {
    const TOOLBAR_H_LOGICAL: f64 = 56.0;
    const TOOLBAR_MIN_W_LOGICAL: f64 = 640.0;
    /// Transparent margin around the image so the outer glow is not clipped
    /// by the window edge.
    const GLOW_PAD_LOGICAL: f64 = 12.0;

    let monitor =
        monitor_containing(app, rect).ok_or_else(|| "no monitor for capture rect".to_string())?;
    let sf = monitor.scale_factor();
    let (mx, my) = (monitor.position().x as f64, monitor.position().y as f64);
    let (mw, mh) = (monitor.size().width as f64, monitor.size().height as f64);

    let toolbar_h = TOOLBAR_H_LOGICAL * sf;
    let toolbar_w = TOOLBAR_MIN_W_LOGICAL * sf;

    let scale = 1.0f64
        .min(mw / rect.width.max(1) as f64)
        .min((mh - toolbar_h).max(1.0) / rect.height.max(1) as f64);
    let pw = ((rect.width as f64 * scale).round() as i32).max(1);
    let ph = ((rect.height as f64 * scale).round() as i32).max(1);
    let inner_w = pw.max(toolbar_w as i32) as f64;
    let inner_h = ph as f64 + toolbar_h;
    let pad = GLOW_PAD_LOGICAL * sf;
    // The glow margin shrinks when the capture already fills the monitor so
    // the window never exceeds the screen (clamp would panic otherwise).
    let win_w = (inner_w + 2.0 * pad).min(mw);
    let win_h = (inner_h + 2.0 * pad).min(mh);
    let px = (rect.x as f64 - pad).clamp(mx, mx + mw - win_w);
    let py = (rect.y as f64 - pad).clamp(my, my + mh - win_h);
    // Canvas offset inside the window (device px): the image must still sit
    // exactly over the captured screen region.
    let ox = rect.x as f64 - px;
    let oy = rect.y as f64 - py;

    eprintln!(
        "open_editor_window: rect {}x{} at ({},{}) sf={:.2} -> image {}x{} win {}x{} at ({},{}) offset ({},{})",
        rect.width,
        rect.height,
        rect.x,
        rect.y,
        sf,
        pw,
        ph,
        win_w as i32,
        win_h as i32,
        px as i32,
        py as i32,
        ox as i32,
        oy as i32
    );

    let url = format!(
        "index.html?editor=1&path={}&x={}&y={}&w={}&h={}&ox={}&oy={}",
        urlencoding::encode(path),
        px as i32,
        py as i32,
        pw,
        ph,
        ox as i32,
        oy as i32,
    );

    let label = format!(
        "editor-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    let owner = app.clone();
    let title =
        crate::i18n::tr(crate::i18n::resolve_lang(), crate::i18n::Str::EditorTitle).to_string();
    app.run_on_main_thread(move || {
        if let Err(e) =
            tauri::WebviewWindowBuilder::new(&owner, &label, tauri::WebviewUrl::App(url.into()))
                .title(title)
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .inner_size(win_w / sf, win_h / sf)
                .position(px / sf, py / sf)
                .build()
        {
            eprintln!("open_editor_window build failed: {e}");
        }
    })
    .map_err(|e| e.to_string())
}

fn monitor_containing(app: &tauri::AppHandle, rect: &Rect) -> Option<tauri::Monitor> {
    let cx = rect.x + rect.width as i32 / 2;
    let cy = rect.y + rect.height as i32 / 2;
    let monitors = app.available_monitors().ok()?;
    monitors
        .into_iter()
        .find(|m| {
            let (mx, my) = (m.position().x, m.position().y);
            let (mw, mh) = (m.size().width as i32, m.size().height as i32);
            cx >= mx && cx < mx + mw && cy >= my && cy < my + mh
        })
        .or_else(|| app.primary_monitor().ok().flatten())
}

#[tauri::command]
pub async fn save_annotated_image(path: String, data: Vec<u8>) -> Result<(), String> {
    tokio::task::spawn_blocking(move || std::fs::write(&path, &data).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn copy_image_to_clipboard(path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let img = image::open(&path).map_err(|e| e.to_string())?.to_rgba8();
        let backend = platform_windows::clipboard::WindowsClipboardBackend::new();
        use clipboard_core::ClipboardBackend;
        backend.set_image(&img).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn read_file_bytes(path: String) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || std::fs::read(&path).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}
