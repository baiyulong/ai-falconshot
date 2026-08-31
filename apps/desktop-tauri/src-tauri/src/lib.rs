mod commands;

use commands::{ai, capture, history, ocr, settings, window};
use settings_core::{JsonSettingsBackend, SettingsBackend};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

/// (Re-)register the global screenshot hotkey. An empty string unregisters
/// (used while hotkeys are paused).
pub fn apply_screenshot_hotkey(app: &tauri::AppHandle, hotkey: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let gs = app.global_shortcut();
    gs.unregister_all().map_err(|e| e.to_string())?;
    let hotkey = hotkey.trim();
    if hotkey.is_empty() {
        return Ok(());
    }
    let shortcut: tauri_plugin_global_shortcut::Shortcut = hotkey
        .parse()
        .map_err(|_| format!("无效的快捷键: {hotkey}"))?;
    gs.register(shortcut).map_err(|e| e.to_string())
}

fn initial_screenshot_hotkey() -> String {
    JsonSettingsBackend::new(JsonSettingsBackend::default_path())
        .load()
        .map(|s| s.hotkeys.screenshot)
        .unwrap_or_else(|_| "F2".to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if capture::CAPTURE_BUSY.swap(true, std::sync::atomic::Ordering::SeqCst) {
                        return;
                    }
                    let app = app.clone();
                    std::thread::spawn(move || {
                        let _ = capture::capture_and_edit(&app);
                        capture::CAPTURE_BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
                    });
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            capture::start_capture,
            capture::capture_region,
            capture::capture_fullscreen,
            capture::save_annotated_image,
            capture::copy_image_to_clipboard,
            capture::read_file_bytes,
            ocr::run_ocr,
            ai::ai_extract,
            settings::get_settings,
            settings::save_settings,
            history::get_history,
            history::clear_history,
            window::start_window_drag,
            window::close_editor,
            window::pin_image,
        ])
        .setup(|app| {
            let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let icon_bytes = include_bytes!("../icons/icon.png");
            let icon_img = image::load_from_memory(icon_bytes)
                .expect("failed to decode tray icon")
                .to_rgba8();
            let (w, h) = icon_img.dimensions();
            let tray_icon = tauri::image::Image::new_owned(icon_img.into_raw(), w, h);

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&menu)
                .tooltip("FalconShot")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                // Dev convenience: show the main window immediately.
                let _ = window.show();
                let _ = window.set_focus();
            }

            // Register the configured global screenshot hotkey (F2 default).
            let hotkey = initial_screenshot_hotkey();
            if let Err(e) = apply_screenshot_hotkey(app.handle(), &hotkey) {
                eprintln!("register screenshot hotkey '{hotkey}' failed: {e}");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running FalconShot");
}
