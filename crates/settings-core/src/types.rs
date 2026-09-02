use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub appearance: AppearanceSettings,
    pub capture: CaptureSettings,
    pub pin: PinSettings,
    pub ocr: OcrSettings,
    pub ai: AiSettings,
    pub hotkeys: HotkeySettings,
    pub privacy: PrivacySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub launch_on_startup: bool,
    pub language: String,
    pub default_save_dir: String,
    pub default_image_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub theme: ThemeMode,
    pub accent_color: String,
    pub mask_color: [u8; 4],
    pub pin_border: bool,
    pub pin_shadow: bool,
    pub pin_corner_radius: u32,
    pub pin_default_opacity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSettings {
    pub default_action: CaptureAction,
    pub show_magnifier: bool,
    pub show_dimensions: bool,
    pub remember_last_region: bool,
    pub auto_detect_window: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureAction {
    Copy,
    Save,
    Pin,
    Ocr,
    AiAnalyze,
    Annotate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinSettings {
    pub default_opacity: f32,
    pub default_always_on_top: bool,
    pub restore_on_launch: bool,
    pub save_on_exit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrSettings {
    pub engine: OcrEngine,
    pub default_language: String,
    pub auto_copy: bool,
    pub preprocess: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcrEngine {
    Local,
    Cloud,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    pub provider: String,
    pub model: String,
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub system_prompt: String,
    pub timeout_secs: u32,
    pub allow_image_upload: bool,
    pub save_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeySettings {
    #[serde(default = "default_screenshot_hotkey")]
    pub screenshot: String,
    pub bindings: Vec<serde_json::Value>,
    pub paused: bool,
}

fn default_screenshot_hotkey() -> String {
    "F2".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub privacy_mode: bool,
    pub allow_cloud: bool,
    pub encrypt_history: bool,
    pub auto_redact: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            general: GeneralSettings {
                launch_on_startup: false,
                // "system" follows the OS UI language; resolved to
                // "zh-CN"/"en" by the app layers.
                language: "system".to_string(),
                default_save_dir: String::new(),
                default_image_format: "png".to_string(),
            },
            appearance: AppearanceSettings {
                theme: ThemeMode::System,
                accent_color: "#00AEFF".to_string(),
                mask_color: [0, 0, 0, 100],
                pin_border: true,
                pin_shadow: true,
                pin_corner_radius: 4,
                pin_default_opacity: 1.0,
            },
            capture: CaptureSettings {
                default_action: CaptureAction::Copy,
                show_magnifier: true,
                show_dimensions: true,
                remember_last_region: true,
                auto_detect_window: true,
            },
            pin: PinSettings {
                default_opacity: 1.0,
                default_always_on_top: true,
                restore_on_launch: true,
                save_on_exit: true,
            },
            ocr: OcrSettings {
                engine: OcrEngine::Local,
                default_language: "zh-CN".to_string(),
                auto_copy: false,
                preprocess: true,
            },
            ai: AiSettings {
                provider: "openai_compatible".to_string(),
                model: "deepseek-chat".to_string(),
                base_url: Some("https://api.deepseek.com".to_string()),
                api_key: String::new(),
                system_prompt: String::new(),
                timeout_secs: 60,
                allow_image_upload: false,
                save_history: true,
            },
            hotkeys: HotkeySettings {
                screenshot: "F2".to_string(),
                bindings: Vec::new(),
                paused: false,
            },
            privacy: PrivacySettings {
                privacy_mode: false,
                allow_cloud: false,
                encrypt_history: false,
                auto_redact: false,
            },
        }
    }
}
