#[tauri::command]
pub async fn get_history(limit: usize, offset: usize) -> Result<String, String> {
    let _ = (limit, offset);
    // TODO: Query history via storage-core
    Ok("[]".to_string())
}

#[tauri::command]
pub async fn clear_history() -> Result<(), String> {
    // TODO: Clear history via storage-core
    Ok(())
}
