use crate::apply_screenshot_hotkey;
use settings_core::{AppSettings, JsonSettingsBackend, SettingsBackend};
use tauri::AppHandle;

fn backend() -> JsonSettingsBackend {
    JsonSettingsBackend::new(JsonSettingsBackend::default_path())
}

#[tauri::command]
pub async fn get_settings() -> Result<String, String> {
    let settings = backend().load().map_err(|e| e.to_string())?;
    serde_json::to_string(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings_json: String) -> Result<(), String> {
    let settings: AppSettings = serde_json::from_str(&settings_json).map_err(|e| e.to_string())?;
    let lang_before = crate::i18n::resolve_lang();
    backend().save(&settings).map_err(|e| e.to_string())?;
    let hotkey = if settings.hotkeys.paused {
        "" // clears the registration while hotkeys are paused
    } else {
        &settings.hotkeys.screenshot
    };
    apply_screenshot_hotkey(&app, hotkey)?;

    // The tray menu is plain text, so rebuild it when the UI language changed
    // (resolve_lang re-reads the just-saved settings).
    let lang_after = crate::i18n::resolve_lang();
    if lang_after != lang_before {
        if let Some(tray) = app.tray_by_id("main") {
            let menu = crate::build_tray_menu(&app, lang_after).map_err(|e| e.to_string())?;
            tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
