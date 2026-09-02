import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import AnnotationEditor from "./components/AnnotationEditor";
import { syncLanguageFromSettings } from "./i18n";

type Page = "settings" | "history" | "ocr";

const isEditorWindow = new URLSearchParams(window.location.search).get("editor") === "1";

function App() {
  if (isEditorWindow) {
    return <AnnotationEditor />;
  }
  return <MainApp />;
}

function MainApp() {
  const { t } = useTranslation();
  const [currentPage, setCurrentPage] = useState<Page>("settings");
  const [captureStatus, setCaptureStatus] = useState("");

  useEffect(() => {
    void syncLanguageFromSettings();
  }, []);

  const startCapture = async () => {
    setCaptureStatus(t("capture.selecting"));
    try {
      await invoke("start_capture");
      setCaptureStatus("");
    } catch (e) {
      setCaptureStatus(
        String(e) === "cancelled" ? t("capture.cancelled") : t("capture.failed", { message: String(e) })
      );
      setTimeout(() => setCaptureStatus(""), 5000);
    }
  };

  const fullscreenCapture = async () => {
    setCaptureStatus(t("capture.capturing"));
    try {
      await invoke("capture_fullscreen");
      setCaptureStatus("");
    } catch (e) {
      setCaptureStatus(t("capture.failed", { message: String(e) }));
      setTimeout(() => setCaptureStatus(""), 5000);
    }
  };

  return (
    <div className="flex h-screen bg-gray-50 dark:bg-gray-900">
      <nav className="w-48 border-r border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 flex flex-col">
        <h1 className="text-lg font-bold text-primary mb-6">FalconShot</h1>
        <ul className="space-y-2">
          {(["settings", "history", "ocr"] as Page[]).map((page) => (
            <li key={page}>
              <button
                onClick={() => setCurrentPage(page)}
                className={`w-full text-left px-3 py-2 rounded-md text-sm ${
                  currentPage === page
                    ? "bg-primary/10 text-primary font-medium"
                    : "text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
                }`}
              >
                {page === "settings" && t("nav.settings")}
                {page === "history" && t("nav.history")}
                {page === "ocr" && t("nav.ocr")}
              </button>
            </li>
          ))}
        </ul>

        <div className="mt-auto space-y-2 pt-4 border-t border-gray-200 dark:border-gray-700">
          <button
            onClick={startCapture}
            className="w-full px-3 py-2 bg-primary text-white rounded-md text-sm hover:bg-primary/90"
          >
            {t("capture.region")}
          </button>
          <button
            onClick={fullscreenCapture}
            className="w-full px-3 py-2 bg-gray-600 text-white rounded-md text-sm hover:bg-gray-500"
          >
            {t("capture.fullscreen")}
          </button>
          {captureStatus && (
            <p className="text-xs text-gray-500 dark:text-gray-400 break-all">{captureStatus}</p>
          )}
        </div>
      </nav>
      <main className="flex-1 p-6 overflow-auto">
        {currentPage === "settings" && <SettingsPage />}
        {currentPage === "history" && <HistoryPage />}
        {currentPage === "ocr" && <OcrPage />}
      </main>
    </div>
  );
}

interface OcrBlock {
  text: string;
  bbox: number[];
  confidence: number;
  block_type: string;
}

interface OcrResult {
  text: string;
  language: string;
  blocks: OcrBlock[];
  confidence: number;
  duration_ms: number;
}

function OcrPage() {
  const { t } = useTranslation();
  const [imagePath, setImagePath] = useState("");
  const [result, setResult] = useState<OcrResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);

  const runOcr = async () => {
    if (!imagePath.trim()) return;
    setLoading(true);
    setError("");
    setResult(null);
    try {
      const json = await invoke<string>("run_ocr", { imagePath });
      setResult(JSON.parse(json));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const copyText = async () => {
    if (!result) return;
    await navigator.clipboard.writeText(result.text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div>
      <h2 className="text-xl font-semibold mb-4 text-gray-800 dark:text-gray-100">{t("ocr.title")}</h2>

      <div className="flex gap-2 mb-4">
        <input
          type="text"
          value={imagePath}
          onChange={(e) => setImagePath(e.target.value)}
          placeholder={t("ocr.placeholder")}
          className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100 text-sm"
          onKeyDown={(e) => e.key === "Enter" && runOcr()}
        />
        <button
          onClick={runOcr}
          disabled={loading}
          className="px-4 py-2 bg-primary text-white rounded-md text-sm hover:bg-primary/90 disabled:opacity-50"
        >
          {loading ? t("ocr.running") : t("ocr.run")}
        </button>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md text-red-600 dark:text-red-400 text-sm">
          {error}
        </div>
      )}

      {result && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="text-sm text-gray-500 dark:text-gray-400">
              {t("ocr.meta", {
                language: result.language,
                confidence: (result.confidence * 100).toFixed(0),
                duration: result.duration_ms,
              })}
            </div>
            <button
              onClick={copyText}
              className="px-3 py-1 text-sm bg-gray-100 dark:bg-gray-700 rounded-md hover:bg-gray-200 dark:hover:bg-gray-600"
            >
              {copied ? t("common.copied") : t("ocr.copy")}
            </button>
          </div>

          <div className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md">
            <pre className="whitespace-pre-wrap text-sm text-gray-800 dark:text-gray-100 font-mono">
              {result.text}
            </pre>
          </div>

          {result.blocks.length > 0 && (
            <div className="border border-gray-200 dark:border-gray-700 rounded-md overflow-hidden">
              <table className="w-full text-sm">
                <thead className="bg-gray-50 dark:bg-gray-800">
                  <tr>
                    <th className="text-left px-3 py-2 text-gray-500">{t("ocr.colType")}</th>
                    <th className="text-left px-3 py-2 text-gray-500">{t("ocr.colContent")}</th>
                    <th className="text-right px-3 py-2 text-gray-500">{t("ocr.colConfidence")}</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100 dark:divide-gray-700">
                  {result.blocks.map((block, i) => (
                    <tr key={i}>
                      <td className="px-3 py-2 text-gray-500">{block.block_type}</td>
                      <td className="px-3 py-2 text-gray-800 dark:text-gray-100 max-w-md truncate">{block.text}</td>
                      <td className="px-3 py-2 text-right text-gray-500">{(block.confidence * 100).toFixed(0)}%</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

interface HistoryEntry {
  id: string;
  entry_type: string;
  title: string;
  created_at: string;
  tags: string[];
}

function HistoryPage() {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<string>("get_history", { limit: 50, offset: 0 })
      .then((json) => setEntries(JSON.parse(json)))
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  const clearAll = async () => {
    await invoke("clear_history");
    setEntries([]);
  };

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-gray-800 dark:text-gray-100">{t("history.title")}</h2>
        {entries.length > 0 && (
          <button
            onClick={clearAll}
            className="px-3 py-1 text-sm text-red-500 border border-red-200 dark:border-red-800 rounded-md hover:bg-red-50 dark:hover:bg-red-900/20"
          >
            {t("history.clear")}
          </button>
        )}
      </div>

      {loading ? (
        <p className="text-gray-500">{t("common.loading")}</p>
      ) : entries.length === 0 ? (
        <p className="text-gray-500 dark:text-gray-400">{t("history.empty")}</p>
      ) : (
        <div className="space-y-2">
          {entries.map((entry) => (
            <div
              key={entry.id}
              className="p-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md"
            >
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-gray-800 dark:text-gray-100">{entry.title}</span>
                <span className="text-xs text-gray-400">{entry.created_at}</span>
              </div>
              <div className="flex gap-1 mt-1">
                <span className="text-xs px-1.5 py-0.5 bg-gray-100 dark:bg-gray-700 rounded text-gray-500">
                  {entry.entry_type}
                </span>
                {entry.tags.map((tag) => (
                  <span key={tag} className="text-xs px-1.5 py-0.5 bg-primary/10 rounded text-primary">
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function SettingsPage() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<Record<string, unknown> | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    invoke<string>("get_settings")
      .then((json) => setSettings(JSON.parse(json)))
      .catch(() => {});
  }, []);

  const save = async () => {
    if (!settings) return;
    await invoke("save_settings", { settingsJson: JSON.stringify(settings) });
    // A persisted language change (including "system") applies immediately.
    void syncLanguageFromSettings();
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  if (!settings) return <p className="text-gray-500">{t("common.loading")}</p>;

  const hotkeys = (settings.hotkeys ?? {}) as { screenshot?: string; paused?: boolean };
  const updateHotkeys = (patch: { screenshot?: string; paused?: boolean }) => {
    setSettings({ ...settings, hotkeys: { ...hotkeys, ...patch } });
  };
  const general = (settings.general ?? {}) as { language?: string };
  const updateGeneral = (patch: Partial<{ language: string }>) => {
    setSettings({ ...settings, general: { ...general, ...patch } });
  };
  const ai = (settings.ai ?? {}) as {
    api_key?: string;
    base_url?: string;
    model?: string;
    system_prompt?: string;
  };
  const updateAi = (
    patch: Partial<{ api_key: string; base_url: string; model: string; system_prompt: string }>
  ) => {
    setSettings({ ...settings, ai: { ...ai, ...patch } });
  };

  // Common vision-capable providers; picking one fills the Base URL.
  const PROVIDERS: { id: string; label: string; baseUrl: string; modelHint: string }[] = [
    { id: "agnes", label: t("settings.providerAgnes"), baseUrl: "https://api.agnes-ai.cn/v1", modelHint: "agnes-2.5-flash" },
    { id: "dashscope", label: t("settings.providerDashscope"), baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1", modelHint: "qwen-vl-plus" },
    { id: "zhipu", label: t("settings.providerZhipu"), baseUrl: "https://open.bigmodel.cn/api/paas/v4", modelHint: "glm-4v-flash" },
    { id: "openai", label: t("settings.providerOpenai"), baseUrl: "https://api.openai.com/v1", modelHint: "gpt-4o" },
    { id: "custom", label: t("settings.providerCustom"), baseUrl: "", modelHint: t("settings.customModelHint") },
  ];
  const activeProvider =
    PROVIDERS.find((p) => p.baseUrl !== "" && p.baseUrl === (ai.base_url ?? ""))?.id ?? "custom";

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-gray-800 dark:text-gray-100">{t("settings.title")}</h2>
        <button
          onClick={save}
          className="px-4 py-2 bg-primary text-white rounded-md text-sm hover:bg-primary/90"
        >
          {saved ? t("settings.saved") : t("settings.save")}
        </button>
      </div>

      <div className="space-y-6 max-w-lg">
        <section>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">{t("settings.hotkeys")}</h3>
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600 dark:text-gray-300 whitespace-nowrap">{t("settings.screenshotLabel")}</span>
              <input
                type="text"
                value={hotkeys.screenshot ?? "F2"}
                onChange={(e) => updateHotkeys({ screenshot: e.target.value })}
                placeholder="F2"
                className="w-40 px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100"
              />
            </div>
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input
                type="checkbox"
                checked={hotkeys.paused ?? false}
                onChange={(e) => updateHotkeys({ paused: e.target.checked })}
                className="rounded"
              />
              {t("settings.pauseHotkeys")}
            </label>
            <p className="text-xs text-gray-400">{t("settings.hotkeyHint")}</p>
          </div>
        </section>

        <section>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">
            {t("settings.aiSection")}
          </h3>
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">{t("settings.provider")}</label>
              <select
                value={activeProvider}
                onChange={(e) => {
                  const p = PROVIDERS.find((x) => x.id === e.target.value);
                  if (!p) return;
                  updateAi({ base_url: p.baseUrl, model: ai.model || p.modelHint });
                }}
                className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100"
              >
                {PROVIDERS.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.label}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">{t("settings.apiKey")}</label>
              <input
                type="password"
                value={ai.api_key ?? ""}
                onChange={(e) => updateAi({ api_key: e.target.value })}
                placeholder="sk-..."
                className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">{t("settings.baseUrl")}</label>
              <input
                type="text"
                value={ai.base_url ?? ""}
                onChange={(e) => updateAi({ base_url: e.target.value })}
                placeholder="https://api.agnes-ai.cn/v1"
                className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100"
              />
              <p className="text-xs text-gray-400 mt-1">
                {t("settings.baseUrlHint")}
              </p>
            </div>
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">{t("settings.model")}</label>
              <input
                type="text"
                value={ai.model ?? ""}
                onChange={(e) => updateAi({ model: e.target.value })}
                placeholder={t("settings.modelPlaceholder")}
                className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                {t("settings.systemPrompt")}
              </label>
              <textarea
                value={ai.system_prompt ?? ""}
                onChange={(e) => updateAi({ system_prompt: e.target.value })}
                rows={3}
                className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100 resize-none"
                placeholder={t("settings.systemPromptPlaceholder")}
              />
            </div>
          </div>
        </section>

        <section>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">{t("settings.general")}</h3>
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600 dark:text-gray-300 whitespace-nowrap">{t("settings.language")}:</span>
              <select
                value={general.language ?? "system"}
                onChange={(e) => updateGeneral({ language: e.target.value })}
                className="px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100"
              >
                <option value="system">{t("settings.languageSystem")}</option>
                <option value="zh-CN">{t("settings.languageZh")}</option>
                <option value="en">{t("settings.languageEn")}</option>
              </select>
            </div>
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input type="checkbox" defaultChecked={false} className="rounded" />
              {t("settings.launchOnStartup")}
            </label>
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600 dark:text-gray-300">{t("settings.defaultFormat")}</span>
              <select className="px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700">
                <option>PNG</option>
                <option>JPEG</option>
                <option>WebP</option>
              </select>
            </div>
          </div>
        </section>

        <section>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">{t("settings.captureSection")}</h3>
          <div className="space-y-2">
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input type="checkbox" defaultChecked className="rounded" />
              {t("settings.showMagnifier")}
            </label>
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input type="checkbox" defaultChecked className="rounded" />
              {t("settings.showDimensions")}
            </label>
          </div>
        </section>

        <section>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">{t("settings.pinSection")}</h3>
          <div className="space-y-2">
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input type="checkbox" defaultChecked className="rounded" />
              {t("settings.restorePins")}
            </label>
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input type="checkbox" defaultChecked className="rounded" />
              {t("settings.savePinState")}
            </label>
          </div>
        </section>
      </div>
    </div>
  );
}

export default App;
