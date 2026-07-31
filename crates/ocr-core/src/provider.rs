use crate::types::{OcrRequest, OcrResult};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait OcrProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_local(&self) -> bool;
    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult>;
    fn supported_languages(&self) -> Vec<String>;
}
