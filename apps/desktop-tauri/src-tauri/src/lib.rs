mod commands;
mod i18n;

use commands::{ai, capture, history, ocr, settings, window};
use i18n::{Lang, Str};
use settings_core::{JsonSettingsBackend, SettingsBackend};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

/// Physical size the taskbar draws tray icons at, straight from the OS —
/// the authoritative value (16 logical px × monitor DPI in a PMv2 process),
/// no guessing from monitor APIs.
#[cfg(windows)]
fn tray_icon_size() -> u32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};
    unsafe { GetSystemMetrics(SM_CXSMICON).max(16) as u32 }
}

#[cfg(not(windows))]
fn tray_icon_size() -> u32 {
    32
}

/// Build the tray menu for the given language. Rebuilt at setup and again in
/// `save_settings` when the user changes the UI language.
pub fn build_tray_menu(app: &tauri::AppHandle, lang: Lang) -> tauri::Result<Menu<tauri::Wry>> {
    let show_item = MenuItem::with_id(
        app,
        "show",
        i18n::tr(lang, Str::TrayShow),
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(
        app,
        "quit",
        i18n::tr(lang, Str::TrayQuit),
        true,
        None::<&str>,
    )?;
    Menu::with_items(app, &[&show_item, &quit_item])
}

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
        .map_err(|_| i18n::hotkey_invalid(i18n::resolve_lang(), hotkey))?;
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
            let lang = i18n::resolve_lang();
            let menu = build_tray_menu(app.handle(), lang)?;

            // tray.png is a small-size optimized variant of the logo (tight
            // crop, posterized colors) — gradients from the 1024px master
            // turn to mush at tray sizes. Pre-scale with Lanczos to exactly
            // the size the taskbar uses (SM_CXSMICON) so the OS never
            // stretches it.
            let tray_size = tray_icon_size();
            eprintln!("tray icon size: {tray_size}px");
            let icon_bytes = include_bytes!("../icons/tray.png");
            let icon_img = image::load_from_memory(icon_bytes)
                .expect("failed to decode tray icon")
                .resize_exact(tray_size, tray_size, image::imageops::FilterType::Lanczos3)
                .to_rgba8();
            let (w, h) = icon_img.dimensions();
            let tray_icon = tauri::image::Image::new_owned(icon_img.into_raw(), w, h);

            let _tray = TrayIconBuilder::with_id("main")
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
