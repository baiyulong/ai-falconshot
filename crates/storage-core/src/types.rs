use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub entry_type: HistoryType,
    pub title: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryType {
    Screenshot,
    Pin,
    Ocr,
    AiAnalysis,
    ColorPick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: String,
    pub max_history_days: u32,
    pub max_storage_mb: u64,
    pub auto_cleanup: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: String::new(),
            max_history_days: 30,
            max_storage_mb: 500,
            auto_cleanup: true,
        }
    }
}
