import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import AnnotationEditor from "./components/AnnotationEditor";

type Page = "settings" | "history" | "ocr";

const isEditorWindow = new URLSearchParams(window.location.search).get("editor") === "1";

function App() {
  if (isEditorWindow) {
    return <AnnotationEditor />;
  }
  return <MainApp />;
}

function MainApp() {
  const [currentPage, setCurrentPage] = useState<Page>("settings");
  const [captureStatus, setCaptureStatus] = useState("");

  const startCapture = async () => {
    setCaptureStatus("选区中...");
    try {
      await invoke("start_capture");
      setCaptureStatus("");
    } catch (e) {
      setCaptureStatus(String(e) === "cancelled" ? "已取消" : `失败: ${e}`);
      setTimeout(() => setCaptureStatus(""), 5000);
    }
  };

  const fullscreenCapture = async () => {
    setCaptureStatus("截图中...");
    try {
      await invoke("capture_fullscreen");
      setCaptureStatus("");
    } catch (e) {
      setCaptureStatus(`失败: ${e}`);
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
                {page === "settings" && "设置"}
                {page === "history" && "历史记录"}
                {page === "ocr" && "提取文本"}
              </button>
            </li>
          ))}
        </ul>

        <div className="mt-auto space-y-2 pt-4 border-t border-gray-200 dark:border-gray-700">
          <button
            onClick={startCapture}
            className="w-full px-3 py-2 bg-primary text-white rounded-md text-sm hover:bg-primary/90"
          >
            区域截图
          </button>
          <button
            onClick={fullscreenCapture}
            className="w-full px-3 py-2 bg-gray-600 text-white rounded-md text-sm hover:bg-gray-500"
          >
            全屏截图
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
      <h2 className="text-xl font-semibold mb-4 text-gray-800 dark:text-gray-100">提取文本</h2>

      <div className="flex gap-2 mb-4">
        <input
          type="text"
          value={imagePath}
          onChange={(e) => setImagePath(e.target.value)}
          placeholder="输入图片路径..."
          className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100 text-sm"
          onKeyDown={(e) => e.key === "Enter" && runOcr()}
        />
        <button
          onClick={runOcr}
          disabled={loading}
          className="px-4 py-2 bg-primary text-white rounded-md text-sm hover:bg-primary/90 disabled:opacity-50"
        >
          {loading ? "识别中..." : "识别"}
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
              语言: {result.language} | 置信度: {(result.confidence * 100).toFixed(0)}% | 耗时: {result.duration_ms}ms
            </div>
            <button
              onClick={copyText}
              className="px-3 py-1 text-sm bg-gray-100 dark:bg-gray-700 rounded-md hover:bg-gray-200 dark:hover:bg-gray-600"
            >
              {copied ? "已复制" : "复制文本"}
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
                    <th className="text-left px-3 py-2 text-gray-500">类型</th>
                    <th className="text-left px-3 py-2 text-gray-500">内容</th>
                    <th className="text-right px-3 py-2 text-gray-500">置信度</th>
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
        <h2 className="text-xl font-semibold text-gray-800 dark:text-gray-100">历史记录</h2>
        {entries.length > 0 && (
          <button
            onClick={clearAll}
            className="px-3 py-1 text-sm text-red-500 border border-red-200 dark:border-red-800 rounded-md hover:bg-red-50 dark:hover:bg-red-900/20"
          >
            清空
          </button>
        )}
      </div>

      {loading ? (
        <p className="text-gray-500">加载中...</p>
      ) : entries.length === 0 ? (
        <p className="text-gray-500 dark:text-gray-400">暂无历史记录</p>
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
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  if (!settings) return <p className="text-gray-500">加载中...</p>;

  const hotkeys = (settings.hotkeys ?? {}) as { screenshot?: string; paused?: boolean };
  const updateHotkeys = (patch: { screenshot?: string; paused?: boolean }) => {
    setSettings({ ...settings, hotkeys: { ...hotkeys, ...patch } });
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
    { id: "agnes", label: "Agnes AI（推荐 agnes-2.5-flash，免费）", baseUrl: "https://api.agnes-ai.cn/v1", modelHint: "agnes-2.5-flash" },
    { id: "dashscope", label: "阿里云百炼（qwen-vl 系列）", baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1", modelHint: "qwen-vl-plus" },
    { id: "zhipu", label: "智谱开放平台（glm-4v 系列）", baseUrl: "https://open.bigmodel.cn/api/paas/v4", modelHint: "glm-4v-flash" },
    { id: "openai", label: "OpenAI（gpt-4o 系列）", baseUrl: "https://api.openai.com/v1", modelHint: "gpt-4o" },
    { id: "custom", label: "自定义（OpenAI 兼容 /v1 根地址）", baseUrl: "", modelHint: "支持图片输入的模型" },
  ];
  const activeProvider =
    PROVIDERS.find((p) => p.baseUrl !== "" && p.baseUrl === (ai.base_url ?? ""))?.id ?? "custom";

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-gray-800 dark:text-gray-100">设置</h2>
        <button
          onClick={save}
          className="px-4 py-2 bg-primary text-white rounded-md text-sm hover:bg-primary/90"
        >
          {saved ? "已保存" : "保存"}
        </button>
      </div>

      <div className="space-y-6 max-w-lg">
        <section>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">快捷键</h3>
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600 dark:text-gray-300 whitespace-nowrap">截图:</span>
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
              暂停快捷键
            </label>
            <p className="text-xs text-gray-400">支持组合键，如 Ctrl+Shift+A。保存后生效。</p>
          </div>
        </section>

        <section>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">
            AI 识别模型（需支持图片输入的 OpenAI 兼容接口）
          </h3>
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">服务商</label>
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
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">API Key</label>
              <input
                type="password"
                value={ai.api_key ?? ""}
                onChange={(e) => updateAi({ api_key: e.target.value })}
                placeholder="sk-..."
                className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">Base URL</label>
              <input
                type="text"
                value={ai.base_url ?? ""}
                onChange={(e) => updateAi({ base_url: e.target.value })}
                placeholder="https://api.agnes-ai.cn/v1"
                className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100"
              />
              <p className="text-xs text-gray-400 mt-1">
                填 API 根地址（到 /v1），不要填 images/generations 等具体端点
              </p>
            </div>
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">模型</label>
              <input
                type="text"
                value={ai.model ?? ""}
                onChange={(e) => updateAi({ model: e.target.value })}
                placeholder="如 qwen-vl-plus / gpt-4o / glm-4v（需支持视觉）"
                className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 dark:text-gray-400 mb-1">
                System Prompt（留空使用默认的 OCR 提示词）
              </label>
              <textarea
                value={ai.system_prompt ?? ""}
                onChange={(e) => updateAi({ system_prompt: e.target.value })}
                rows={3}
                className="w-full px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700 text-gray-800 dark:text-gray-100 resize-none"
                placeholder="默认：精准提取图片文字，Markdown 输出，表格转 Markdown 表格"
              />
            </div>
          </div>
        </section>

        <section>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">通用</h3>
          <div className="space-y-2">
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input type="checkbox" defaultChecked={false} className="rounded" />
              开机自启动
            </label>
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600 dark:text-gray-300">默认保存格式:</span>
              <select className="px-2 py-1 border border-gray-300 dark:border-gray-600 rounded text-sm bg-white dark:bg-gray-700">
                <option>PNG</option>
                <option>JPEG</option>
                <option>WebP</option>
              </select>
            </div>
          </div>
        </section>

        <section>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">截图</h3>
          <div className="space-y-2">
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input type="checkbox" defaultChecked className="rounded" />
              显示放大镜
            </label>
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input type="checkbox" defaultChecked className="rounded" />
              显示尺寸标注
            </label>
          </div>
        </section>

        <section>
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">贴图</h3>
          <div className="space-y-2">
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input type="checkbox" defaultChecked className="rounded" />
              启动时恢复贴图
            </label>
            <label className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-300">
              <input type="checkbox" defaultChecked className="rounded" />
              退出时保存状态
            </label>
          </div>
        </section>
      </div>
    </div>
  );
}

export default App;
