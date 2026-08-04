use capture_core::{CaptureBackend, CaptureOptions, Rect};
use platform_windows::capture::WindowsCaptureBackend;
use platform_windows::overlay::WindowsOverlay;

#[tauri::command]
pub async fn start_capture() -> Result<String, String> {
    let result = tokio::task::spawn_blocking(|| {
        let mut overlay = WindowsOverlay::new();
        overlay.show_and_select()
    })
    .await
    .map_err(|e| e.to_string())?;

    match result {
        Ok(platform_windows::overlay::OverlayResult::Selected(rect)) => {
            let path = tokio::task::spawn_blocking(move || capture_rect_to_file(&rect))
                .await
                .map_err(|e| e.to_string())??;
            Ok(path)
        }
        Ok(platform_windows::overlay::OverlayResult::Cancelled) => Err("cancelled".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<String, String> {
    let rect = Rect::new(x, y, width, height);
    tokio::task::spawn_blocking(move || capture_rect_to_file(&rect))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn capture_fullscreen() -> Result<String, String> {
    tokio::task::spawn_blocking(|| {
        let backend = WindowsCaptureBackend::new().map_err(|e| e.to_string())?;
        let frame = backend.capture_fullscreen().map_err(|e| e.to_string())?;
        save_image(&frame.image)
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
    let frame = backend.capture_region(&options).map_err(|e| e.to_string())?;
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

#[tauri::command]
pub async fn save_annotated_image(path: String, data: Vec<u8>) -> Result<(), String> {
    tokio::task::spawn_blocking(move || std::fs::write(&path, &data).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn copy_image_to_clipboard(path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let img = image::open(&path)
            .map_err(|e| e.to_string())?
            .to_rgba8();
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
