use ocr_core::{OcrProvider, OcrRequest, WindowsOcrProvider};

#[tauri::command]
pub async fn run_ocr(image_path: String) -> Result<String, String> {
    let image_data = std::fs::read(&image_path).map_err(|e| e.to_string())?;

    let provider = WindowsOcrProvider::new();
    let request = OcrRequest {
        image_data,
        languages: vec!["zh-Hans".to_string(), "en".to_string()],
        preprocess: true,
    };

    let result = provider
        .recognize(&request)
        .await
        .map_err(|e| e.to_string())?;

    serde_json::to_string(&result).map_err(|e| e.to_string())
}
