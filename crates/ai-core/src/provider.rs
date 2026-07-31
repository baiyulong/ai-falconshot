use crate::types::{AiRequest, AiResponse};
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

#[async_trait]
pub trait VisionProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_local(&self) -> bool;
    async fn analyze(&self, request: &AiRequest) -> Result<AiResponse>;
    async fn analyze_stream(
        &self,
        request: &AiRequest,
        tx: mpsc::Sender<String>,
    ) -> Result<AiResponse>;
}
