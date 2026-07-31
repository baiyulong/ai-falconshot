#[tauri::command]
pub async fn run_ocr(image_path: String) -> Result<String, String> {
    let _ = image_path;
    // TODO: Run OCR via ocr-core provider
    Ok(r#"{"text": "", "blocks": []}"#.to_string())
}
