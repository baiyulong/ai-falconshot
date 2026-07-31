use crate::types::HistoryEntry;
use anyhow::Result;

pub trait HistoryStore: Send + Sync {
    fn add(&self, entry: &HistoryEntry) -> Result<()>;
    fn get(&self, id: &str) -> Result<Option<HistoryEntry>>;
    fn list(&self, limit: usize, offset: usize) -> Result<Vec<HistoryEntry>>;
    fn search(&self, query: &str) -> Result<Vec<HistoryEntry>>;
    fn delete(&self, id: &str) -> Result<()>;
    fn clear(&self) -> Result<()>;
    fn cleanup(&self, max_age_days: u32, max_size_mb: u64) -> Result<u64>;
}
