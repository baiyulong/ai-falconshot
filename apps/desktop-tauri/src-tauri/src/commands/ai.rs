use ai_core::{AiRequest, OpenAiProvider, VisionProvider};
use ocr_core::{OcrProvider, OcrRequest, WindowsOcrProvider};
use settings_core::{JsonSettingsBackend, SettingsBackend};

#[tauri::command]
pub async fn analyze_image(image_path: String, prompt: String) -> Result<String, String> {
    let image_data = std::fs::read(&image_path).map_err(|e| e.to_string())?;

    let backend = JsonSettingsBackend::new(JsonSettingsBackend::default_path());
    let settings = backend.load().map_err(|e| e.to_string())?;

    let api_key = if !settings.ai.api_key.is_empty() {
        settings.ai.api_key.clone()
    } else {
        std::env::var("FALCONSHOT_AI_KEY").unwrap_or_default()
    };

    if api_key.is_empty() {
        return Err("AI API key 未配置。请在设置页面填写 API Key。".to_string());
    }

    let base_url = settings.ai.base_url.clone().unwrap_or_else(|| {
        std::env::var("FALCONSHOT_AI_URL").unwrap_or_else(|_| "https://api.deepseek.com".to_string())
    });
    let model = if let Ok(m) = std::env::var("FALCONSHOT_AI_MODEL") {
        m
    } else {
        settings.ai.model.clone()
    };

    // 模型不支持图片输入时（如 deepseek-chat），先 OCR 提取文字再做文本分析
    let (final_prompt, vision) = if settings.ai.allow_image_upload {
        (prompt, true)
    } else {
        let ocr_text = ocr_image_text(&image_data).await.unwrap_or_default();
        if ocr_text.trim().is_empty() {
            (
                format!(
                    "{}\n\n（注意：截图中未识别到任何文字，请基于这一情况回答。）",
                    prompt
                ),
                false,
            )
        } else {
            (
                format!(
                    "{}\n\n以下是截图经 OCR 识别出的文字内容：\n{}",
                    prompt, ocr_text
                ),
                false,
            )
        }
    };

    let provider = OpenAiProvider::new(&api_key, &base_url, &model)
        .with_timeout(settings.ai.timeout_secs)
        .with_vision(vision);

    let request = AiRequest {
        image_data,
        prompt: final_prompt,
        system_prompt: Some("你是一个图片内容分析助手。请简洁准确地回答用户的问题。".to_string()),
        conversation_id: None,
        history: vec![],
        max_tokens: Some(2048),
        temperature: Some(0.3),
    };

    let response = provider.analyze(&request).await.map_err(|e| e.to_string())?;

    serde_json::to_string(&response).map_err(|e| e.to_string())
}

async fn ocr_image_text(image_data: &[u8]) -> Result<String, String> {
    let provider = WindowsOcrProvider::new();
    let request = OcrRequest {
        image_data: image_data.to_vec(),
        languages: vec!["zh-Hans".to_string(), "en".to_string()],
        preprocess: true,
    };
    let result = provider.recognize(&request).await.map_err(|e| e.to_string())?;
    Ok(result.text)
}
