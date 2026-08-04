use storage_core::{HistoryStore, JsonHistoryStore};

fn store() -> Result<JsonHistoryStore, String> {
    let dir = settings_core::JsonSettingsBackend::app_data_dir();
    JsonHistoryStore::new(dir.join("history.json")).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_history(limit: usize, offset: usize) -> Result<String, String> {
    let store = store()?;
    let entries = store.list(limit, offset).map_err(|e| e.to_string())?;
    serde_json::to_string(&entries).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_history() -> Result<(), String> {
    let store = store()?;
    store.clear().map_err(|e| e.to_string())
}
