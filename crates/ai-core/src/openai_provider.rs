use crate::provider::VisionProvider;
use crate::types::{AiRequest, AiResponse, ChatRole};
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    model: String,
    timeout_secs: u32,
    vision: bool,
}

impl OpenAiProvider {
    pub fn new(api_key: &str, base_url: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            timeout_secs: 60,
            vision: true,
        }
    }

    pub fn with_timeout(mut self, secs: u32) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn with_vision(mut self, enabled: bool) -> Self {
        self.vision = enabled;
        self
    }

    fn build_messages(&self, request: &AiRequest) -> Vec<serde_json::Value> {
        let mut messages = Vec::new();

        if let Some(sys) = &request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }

        for msg in &request.history {
            let role = match msg.role {
                ChatRole::System => "system",
                ChatRole::User => "user",
                ChatRole::Assistant => "assistant",
            };
            messages.push(serde_json::json!({
                "role": role,
                "content": msg.content
            }));
        }

        if self.vision {
            let base64_image = base64_encode(&request.image_data);
            let image_url = format!("data:image/png;base64,{}", base64_image);

            messages.push(serde_json::json!({
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": request.prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": image_url
                        }
                    }
                ]
            }));
        } else {
            messages.push(serde_json::json!({
                "role": "user",
                "content": request.prompt
            }));
        }

        messages
    }
}

#[async_trait]
impl VisionProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai-compatible"
    }

    fn is_local(&self) -> bool {
        false
    }

    async fn analyze(&self, request: &AiRequest) -> Result<AiResponse> {
        let start = std::time::Instant::now();
        let messages = self.build_messages(request);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let client = reqwest_client(self.timeout_secs)?;
        let url = format!("{}/chat/completions", self.base_url);

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("API error {status}: {text}");
        }

        let json: serde_json::Value = resp.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let tokens = json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32;
        let model = json["model"].as_str().unwrap_or(&self.model).to_string();

        Ok(AiResponse {
            content,
            model,
            tokens_used: tokens,
            duration_ms: start.elapsed().as_millis() as u64,
            conversation_id: request.conversation_id.clone(),
        })
    }

    async fn analyze_stream(
        &self,
        request: &AiRequest,
        tx: mpsc::Sender<String>,
    ) -> Result<AiResponse> {
        let start = std::time::Instant::now();
        let messages = self.build_messages(request);

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });

        let client = reqwest_client(self.timeout_secs)?;
        let url = format!("{}/chat/completions", self.base_url);

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("API error {status}: {text}");
        }

        let mut full_content = String::new();
        let bytes = resp.bytes().await?;
        let text = String::from_utf8_lossy(&bytes);

        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with("data: ") {
                continue;
            }
            let data = &line[6..];
            if data == "[DONE]" {
                break;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
                    full_content.push_str(delta);
                    let _ = tx.send(delta.to_string()).await;
                }
            }
        }

        Ok(AiResponse {
            content: full_content,
            model: self.model.clone(),
            tokens_used: 0,
            duration_ms: start.elapsed().as_millis() as u64,
            conversation_id: request.conversation_id.clone(),
        })
    }
}

fn reqwest_client(timeout_secs: u32) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs as u64))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {e}"))
}

fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        let _ = write!(
            result,
            "{}",
            CHARS[((triple >> 18) & 0x3F) as usize] as char
        );
        let _ = write!(
            result,
            "{}",
            CHARS[((triple >> 12) & 0x3F) as usize] as char
        );
        if chunk.len() > 1 {
            let _ = write!(result, "{}", CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            let _ = write!(result, "{}", CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
