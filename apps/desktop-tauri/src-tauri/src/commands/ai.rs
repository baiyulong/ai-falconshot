use settings_core::{JsonSettingsBackend, SettingsBackend};

const DEFAULT_SYSTEM_PROMPT: &str = "你是一个精准的图像文字识别（OCR）助手。请提取图片中的全部文字内容，保持原有段落与层级结构，使用 Markdown 格式输出（表格转为 Markdown 表格，代码使用代码块）。只输出识别到的内容本身，不要添加任何解释或额外说明。";

const DEFAULT_USER_PROMPT: &str = "请识别这张截图中的文字并以 Markdown 输出。";

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[(triple >> 18) as usize & 0x3F] as char);
        out.push(CHARS[(triple >> 12) as usize & 0x3F] as char);
        out.push(if chunk.len() > 1 {
            CHARS[(triple >> 6) as usize & 0x3F] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[triple as usize & 0x3F] as char
        } else {
            '='
        });
    }
    out
}

/// Send the captured image to an OpenAI-compatible vision model and return
/// the recognized content (Markdown) from the first choice.
#[tauri::command]
pub async fn ai_extract(path: String, prompt: Option<String>) -> Result<String, String> {
    let settings = JsonSettingsBackend::new(JsonSettingsBackend::default_path())
        .load()
        .map_err(|e| e.to_string())?;

    let api_key = if !settings.ai.api_key.is_empty() {
        settings.ai.api_key.clone()
    } else {
        std::env::var("FALCONSHOT_AI_KEY").unwrap_or_default()
    };
    if api_key.is_empty() {
        return Err("请先在设置页填写 API Key（需要支持视觉的模型）".to_string());
    }
    let base_url = settings
        .ai
        .base_url
        .clone()
        .unwrap_or_else(|| "https://api.deepseek.com".to_string());
    let system_prompt = if settings.ai.system_prompt.trim().is_empty() {
        DEFAULT_SYSTEM_PROMPT.to_string()
    } else {
        settings.ai.system_prompt.clone()
    };
    let user_prompt = prompt
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_USER_PROMPT.to_string());

    let image_data = tokio::task::spawn_blocking(move || std::fs::read(&path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let data_url = format!("data:image/png;base64,{}", base64_encode(&image_data));

    let body = serde_json::json!({
        "model": settings.ai.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": [
                { "type": "text", "text": user_prompt },
                { "type": "image_url", "image_url": { "url": data_url } }
            ]}
        ],
        "max_tokens": 4096,
        "temperature": 0.2,
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            settings.ai.timeout_secs.max(10) as u64,
        ))
        .build()
        .map_err(|e| e.to_string())?;

    // Base URL is the API root (e.g. https://host/v1); tolerate a full
    // endpoint pasted in. Image-generation endpoints cannot do OCR.
    let mut url = base_url.trim_end_matches('/').to_string();
    if url.ends_with("/images/generations") {
        return Err(
            "Base URL 指向的是图片生成接口（images/generations），无法用于文字识别。请填写 API 根地址，例如 https://api.agnes-ai.cn/v1"
                .to_string(),
        );
    }
    if !url.ends_with("/chat/completions") {
        url.push_str("/chat/completions");
    }

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    let status = resp.status();
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))?;
    if !status.is_success() {
        let msg = match json["error"]["message"].as_str() {
            Some(m) => m.to_string(),
            None => json.to_string(),
        };
        return Err(format!("API 错误 {status}（POST {url}）: {msg}"));
    }

    json["choices"][0]["message"]["content"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "响应中缺少识别内容".to_string())
}
