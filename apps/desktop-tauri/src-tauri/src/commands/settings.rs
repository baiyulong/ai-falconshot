use settings_core::{AppSettings, JsonSettingsBackend, SettingsBackend};

fn backend() -> JsonSettingsBackend {
    JsonSettingsBackend::new(JsonSettingsBackend::default_path())
}

#[tauri::command]
pub async fn get_settings() -> Result<String, String> {
    let settings = backend().load().map_err(|e| e.to_string())?;
    serde_json::to_string(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_settings(settings_json: String) -> Result<(), String> {
    let settings: AppSettings = serde_json::from_str(&settings_json).map_err(|e| e.to_string())?;
    backend().save(&settings).map_err(|e| e.to_string())
}
