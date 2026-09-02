use settings_core::{JsonSettingsBackend, SettingsBackend};

/// UI languages shipped with the app. zh-CN is the product default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    ZhCn,
    En,
}

/// Source strings for the Rust-side UI (tray menu, editor window title, error
/// messages surfaced in the frontend). Placeholders in `{braces}` are
/// substituted by the helper functions below.
#[derive(Debug, Clone, Copy)]
pub enum Str {
    TrayShow,
    TrayQuit,
    EditorTitle,
    HotkeyInvalid,
    AiNoApiKey,
    AiImageGenUrl,
    AiRequestFailed,
    AiParseFailed,
    AiApiError,
    AiNoContent,
}

pub fn tr(lang: Lang, s: Str) -> &'static str {
    match (lang, s) {
        (Lang::ZhCn, Str::TrayShow) => "显示主窗口",
        (Lang::En, Str::TrayShow) => "Show Main Window",
        (Lang::ZhCn, Str::TrayQuit) => "退出",
        (Lang::En, Str::TrayQuit) => "Quit",
        (Lang::ZhCn, Str::EditorTitle) => "FalconShot 编辑",
        (Lang::En, Str::EditorTitle) => "FalconShot Editor",
        (Lang::ZhCn, Str::HotkeyInvalid) => "无效的快捷键: {hotkey}",
        (Lang::En, Str::HotkeyInvalid) => "Invalid hotkey: {hotkey}",
        (Lang::ZhCn, Str::AiNoApiKey) => "请先在设置页填写 API Key（需要支持视觉的模型）",
        (Lang::En, Str::AiNoApiKey) => {
            "Set an API Key on the Settings page first (a vision-capable model is required)"
        }
        (Lang::ZhCn, Str::AiImageGenUrl) => "Base URL 指向的是图片生成接口（images/generations），无法用于文字识别。请填写 API 根地址，例如 https://api.agnes-ai.cn/v1",
        (Lang::En, Str::AiImageGenUrl) => "The Base URL points to an image-generation endpoint (images/generations), which cannot extract text. Use the API root, e.g. https://api.agnes-ai.cn/v1",
        (Lang::ZhCn, Str::AiRequestFailed) => "请求失败: {error}",
        (Lang::En, Str::AiRequestFailed) => "Request failed: {error}",
        (Lang::ZhCn, Str::AiParseFailed) => "解析响应失败: {error}",
        (Lang::En, Str::AiParseFailed) => "Failed to parse response: {error}",
        (Lang::ZhCn, Str::AiApiError) => "API 错误 {status}（POST {url}）: {message}",
        (Lang::En, Str::AiApiError) => "API error {status} (POST {url}): {message}",
        (Lang::ZhCn, Str::AiNoContent) => "响应中缺少识别内容",
        (Lang::En, Str::AiNoContent) => "No recognized content in the response",
    }
}

fn locale_to_lang(locale: &str) -> Lang {
    if locale.to_ascii_lowercase().starts_with("zh") {
        Lang::ZhCn
    } else {
        Lang::En
    }
}

/// Resolve the UI language: an explicit settings value wins; "system" (the
/// default) or anything unknown follows the OS UI language. Reads the
/// settings from disk on every call — cheap, and callers see fresh values
/// right after a save.
pub fn resolve_lang() -> Lang {
    let setting = JsonSettingsBackend::new(JsonSettingsBackend::default_path())
        .load()
        .map(|s| s.general.language)
        .unwrap_or_default();
    match setting.as_str() {
        "zh-CN" => Lang::ZhCn,
        "en" => Lang::En,
        _ => locale_to_lang(&sys_locale::get_locale().unwrap_or_default()),
    }
}

pub fn hotkey_invalid(lang: Lang, hotkey: &str) -> String {
    tr(lang, Str::HotkeyInvalid).replace("{hotkey}", hotkey)
}

pub fn ai_request_failed(lang: Lang, error: &str) -> String {
    tr(lang, Str::AiRequestFailed).replace("{error}", error)
}

pub fn ai_parse_failed(lang: Lang, error: &str) -> String {
    tr(lang, Str::AiParseFailed).replace("{error}", error)
}

pub fn ai_api_error(lang: Lang, status: &str, url: &str, message: &str) -> String {
    tr(lang, Str::AiApiError)
        .replace("{status}", status)
        .replace("{url}", url)
        .replace("{message}", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tr_covers_both_langs() {
        for s in [
            Str::TrayShow,
            Str::TrayQuit,
            Str::EditorTitle,
            Str::HotkeyInvalid,
            Str::AiNoApiKey,
            Str::AiImageGenUrl,
            Str::AiRequestFailed,
            Str::AiParseFailed,
            Str::AiApiError,
            Str::AiNoContent,
        ] {
            assert!(!tr(Lang::ZhCn, s).is_empty());
            assert!(!tr(Lang::En, s).is_empty());
        }
    }

    #[test]
    fn locale_mapping() {
        assert_eq!(locale_to_lang("zh-CN"), Lang::ZhCn);
        assert_eq!(locale_to_lang("ZH-tw"), Lang::ZhCn);
        assert_eq!(locale_to_lang("en-US"), Lang::En);
        assert_eq!(locale_to_lang(""), Lang::En);
    }

    #[test]
    fn placeholder_substitution() {
        assert_eq!(hotkey_invalid(Lang::En, "F3"), "Invalid hotkey: F3");
        assert_eq!(
            ai_api_error(Lang::ZhCn, "500", "http://x", "boom"),
            "API 错误 500（POST http://x）: boom"
        );
    }
}
