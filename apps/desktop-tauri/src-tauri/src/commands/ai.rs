#[tauri::command]
pub async fn analyze_image(image_path: String, prompt: String) -> Result<String, String> {
    let _ = (image_path, prompt);
    // TODO: Run AI analysis via ai-core provider
    Ok("AI analysis not yet configured".to_string())
}
