use crate::provider::OcrProvider;
use crate::types::{OcrBlock, OcrBlockType, OcrRequest, OcrResult};
use anyhow::Result;
use async_trait::async_trait;

pub struct MockOcrProvider;

impl MockOcrProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockOcrProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OcrProvider for MockOcrProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn is_local(&self) -> bool {
        true
    }

    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        let text = format!("[Mock OCR: {} bytes]", request.image_data.len());
        Ok(OcrResult {
            text: text.clone(),
            language: "en".to_string(),
            blocks: vec![OcrBlock {
                text,
                bbox: [0.0, 0.0, 100.0, 50.0],
                confidence: 0.99,
                block_type: OcrBlockType::Text,
            }],
            tables: vec![],
            confidence: 0.99,
            duration_ms: 1,
        })
    }

    fn supported_languages(&self) -> Vec<String> {
        vec!["en".to_string(), "zh".to_string()]
    }
}
