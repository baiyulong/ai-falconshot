use crate::history::HistoryStore;
use crate::types::HistoryEntry;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct JsonHistoryStore {
    path: PathBuf,
    entries: Mutex<Vec<HistoryEntry>>,
}

impl JsonHistoryStore {
    pub fn new(path: PathBuf) -> Result<Self> {
        let entries = if path.exists() {
            let json = std::fs::read_to_string(&path)?;
            serde_json::from_str(&json).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            path,
            entries: Mutex::new(entries),
        })
    }

    fn persist(&self, entries: &[HistoryEntry]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(entries)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}

impl HistoryStore for JsonHistoryStore {
    fn add(&self, entry: &HistoryEntry) -> Result<()> {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(0, entry.clone());
        self.persist(&entries)
    }

    fn get(&self, id: &str) -> Result<Option<HistoryEntry>> {
        let entries = self.entries.lock().unwrap();
        Ok(entries.iter().find(|e| e.id == id).cloned())
    }

    fn list(&self, limit: usize, offset: usize) -> Result<Vec<HistoryEntry>> {
        let entries = self.entries.lock().unwrap();
        Ok(entries.iter().skip(offset).take(limit).cloned().collect())
    }

    fn search(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        let entries = self.entries.lock().unwrap();
        let query_lower = query.to_lowercase();
        Ok(entries
            .iter()
            .filter(|e| {
                e.title.to_lowercase().contains(&query_lower)
                    || e.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect())
    }

    fn delete(&self, id: &str) -> Result<()> {
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|e| e.id != id);
        self.persist(&entries)
    }

    fn clear(&self) -> Result<()> {
        let mut entries = self.entries.lock().unwrap();
        entries.clear();
        self.persist(&entries)
    }

    fn cleanup(&self, max_age_days: u32, _max_size_mb: u64) -> Result<u64> {
        let mut entries = self.entries.lock().unwrap();
        let before = entries.len() as u64;

        let cutoff = chrono_cutoff(max_age_days);
        entries.retain(|e| e.created_at >= cutoff);

        let removed = before - entries.len() as u64;
        if removed > 0 {
            self.persist(&entries)?;
        }
        Ok(removed)
    }
}

fn chrono_cutoff(days: u32) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let cutoff_secs = now.as_secs().saturating_sub(days as u64 * 86400);
    let days_since_epoch = cutoff_secs / 86400;
    let (y, m, d) = days_to_ymd(days_since_epoch);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut y = 1970u64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let days_in_months: [u64; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1u64;
    for &dim in &days_in_months {
        if remaining < dim {
            break;
        }
        remaining -= dim;
        m += 1;
    }
    (y, m, remaining + 1)
}

fn is_leap(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HistoryType;

    fn temp_history_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "falconshot_test_history_{}.json",
            std::process::id()
        ))
    }

    fn test_entry(id: &str, title: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            entry_type: HistoryType::Screenshot,
            title: title.to_string(),
            file_path: None,
            thumbnail_path: None,
            metadata: serde_json::json!({}),
            created_at: "2026-01-15".to_string(),
            tags: vec!["test".to_string()],
        }
    }

    #[test]
    fn test_add_and_get() {
        let path = temp_history_path();
        let _ = std::fs::remove_file(&path);
        let store = JsonHistoryStore::new(path.clone()).unwrap();

        store.add(&test_entry("1", "First")).unwrap();
        let entry = store.get("1").unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().title, "First");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_list_with_pagination() {
        let path = temp_history_path();
        let _ = std::fs::remove_file(&path);
        let store = JsonHistoryStore::new(path.clone()).unwrap();

        for i in 0..5 {
            store
                .add(&test_entry(&format!("{i}"), &format!("Entry {i}")))
                .unwrap();
        }

        let page = store.list(2, 0).unwrap();
        assert_eq!(page.len(), 2);

        let page2 = store.list(2, 2).unwrap();
        assert_eq!(page2.len(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_search() {
        let path = temp_history_path();
        let _ = std::fs::remove_file(&path);
        let store = JsonHistoryStore::new(path.clone()).unwrap();

        store
            .add(&test_entry("1", "Screenshot of dashboard"))
            .unwrap();
        store.add(&test_entry("2", "Error log capture")).unwrap();

        let results = store.search("dashboard").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_delete() {
        let path = temp_history_path();
        let _ = std::fs::remove_file(&path);
        let store = JsonHistoryStore::new(path.clone()).unwrap();

        store.add(&test_entry("1", "To delete")).unwrap();
        store.delete("1").unwrap();
        assert!(store.get("1").unwrap().is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_clear() {
        let path = temp_history_path();
        let _ = std::fs::remove_file(&path);
        let store = JsonHistoryStore::new(path.clone()).unwrap();

        store.add(&test_entry("1", "A")).unwrap();
        store.add(&test_entry("2", "B")).unwrap();
        store.clear().unwrap();
        assert_eq!(store.list(10, 0).unwrap().len(), 0);
        let _ = std::fs::remove_file(&path);
    }
}
