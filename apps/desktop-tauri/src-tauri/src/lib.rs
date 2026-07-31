mod commands;

use commands::{capture, ocr, ai, settings, history};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            capture::start_capture,
            capture::capture_region,
            ocr::run_ocr,
            ai::analyze_image,
            settings::get_settings,
            settings::save_settings,
            history::get_history,
            history::clear_history,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running FalconShot");
}
