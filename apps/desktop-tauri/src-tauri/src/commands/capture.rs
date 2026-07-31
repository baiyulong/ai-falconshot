#[tauri::command]
pub async fn start_capture() -> Result<String, String> {
    // TODO: Trigger native screenshot overlay
    Ok("capture_started".to_string())
}

#[tauri::command]
pub async fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<String, String> {
    let _ = (x, y, width, height);
    // TODO: Capture specific region via native backend
    Ok("region_captured".to_string())
}
