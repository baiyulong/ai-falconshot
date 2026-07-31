#[tauri::command]
pub async fn get_settings() -> Result<String, String> {
    let settings = settings_core::AppSettings::default();
    serde_json::to_string(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_settings(settings_json: String) -> Result<(), String> {
    let _settings: settings_core::AppSettings =
        serde_json::from_str(&settings_json).map_err(|e| e.to_string())?;
    // TODO: Persist settings via settings-core backend
    Ok(())
}
