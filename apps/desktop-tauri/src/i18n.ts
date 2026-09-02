import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import zhCN from "./locales/zh-CN.json";
import en from "./locales/en.json";

export type UiLanguage = "system" | "zh-CN" | "en";

/// Resolve the UI language from the stored setting. "system" (the default, or
/// anything unknown) follows the OS language; Chinese for the zh* family,
/// English otherwise. Explicit values pass through.
export function resolveLanguage(setting: string | undefined, systemLanguage: string): string {
  if (setting === "zh-CN" || setting === "en") return setting;
  const sys = systemLanguage.toLowerCase();
  return sys.startsWith("zh") ? "zh-CN" : "en";
}

function applyDocumentLang(lang: string) {
  document.documentElement.lang = lang;
}

// Synchronous first paint uses the OS language; the persisted setting is
// applied right after mount via syncLanguageFromSettings().
void i18n.use(initReactI18next).init({
  resources: {
    "zh-CN": { translation: zhCN },
    en: { translation: en },
  },
  lng: resolveLanguage(undefined, navigator.language),
  fallbackLng: "zh-CN",
  interpolation: { escapeValue: false },
});

i18n.on("languageChanged", applyDocumentLang);
applyDocumentLang(i18n.language);

/// Called on mount by the main window and by every editor window (each is an
/// independent WebView): an explicit language in settings wins over the
/// system language used for the first paint.
export async function syncLanguageFromSettings(): Promise<void> {
  try {
    const json = await invoke<string>("get_settings");
    const settings = JSON.parse(json) as { general?: { language?: string } };
    const lang = resolveLanguage(settings.general?.language, navigator.language);
    if (i18n.language !== lang) void i18n.changeLanguage(lang);
  } catch {
    // keep the navigator-language default when settings are unavailable
  }
}
