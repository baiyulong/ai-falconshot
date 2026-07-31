use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRequest {
    pub image_data: Vec<u8>,
    pub languages: Vec<String>,
    pub preprocess: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub language: String,
    pub blocks: Vec<OcrBlock>,
    pub tables: Vec<OcrTable>,
    pub confidence: f32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrBlock {
    pub text: String,
    pub bbox: [f32; 4],
    pub confidence: f32,
    pub block_type: OcrBlockType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcrBlockType {
    Text,
    Table,
    Code,
    Heading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrTable {
    pub rows: Vec<Vec<String>>,
    pub bbox: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcrOutputFormat {
    PlainText,
    Markdown,
    Json,
    Csv,
}
