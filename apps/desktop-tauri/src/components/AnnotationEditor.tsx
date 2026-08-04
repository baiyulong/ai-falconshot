import { useRef, useState, useEffect, useCallback, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";

type AnnotationTool = "rect" | "arrow" | "pen" | "text" | "highlighter";

interface Point {
  x: number;
  y: number;
}

interface AnnotationObj {
  tool: AnnotationTool;
  points: Point[];
  color: string;
  width: number;
  text?: string;
}

interface Props {
  imagePath: string;
  onClose: () => void;
}

interface OcrResult {
  text: string;
  language: string;
  confidence: number;
  duration_ms: number;
}

interface AiResponse {
  content: string;
  model: string;
  tokens_used: number;
  duration_ms: number;
}

const AI_TEMPLATES = [
  { id: "summarize", label: "总结内容", prompt: "请总结这张截图的内容。" },
  { id: "translate", label: "翻译", prompt: "请翻译这张截图中的文字内容。" },
  { id: "error", label: "分析报错", prompt: "请分析这张截图中的报错信息，给出原因分析和排查步骤。" },
  { id: "table", label: "提取表格", prompt: "请提取这张截图中的表格数据，以 Markdown 格式输出。" },
];

const TOOLS: { id: AnnotationTool; icon: ReactNode; title: string }[] = [
  {
    id: "rect",
    title: "矩形",
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
        <rect x="2" y="3" width="12" height="10" rx="1" />
      </svg>
    ),
  },
  {
    id: "arrow",
    title: "箭头",
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
        <line x1="2" y1="14" x2="13" y2="3" />
        <polyline points="8,3 13,3 13,8" />
      </svg>
    ),
  },
  {
    id: "pen",
    title: "画笔",
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path d="M2 14 C4 8, 8 4, 14 2" strokeLinecap="round" />
      </svg>
    ),
  },
  {
    id: "text",
    title: "文字",
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
        <line x1="3" y1="3" x2="13" y2="3" />
        <line x1="8" y1="3" x2="8" y2="13" />
        <line x1="5" y1="13" x2="11" y2="13" />
      </svg>
    ),
  },
  {
    id: "highlighter",
    title: "高亮",
    icon: (
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
        <rect x="2" y="6" width="12" height="5" rx="1" fill="currentColor" opacity="0.4" />
        <line x1="2" y1="13" x2="14" y2="13" />
      </svg>
    ),
  },
];

const COLORS = ["#FF0000", "#00AEFF", "#00CC00", "#FFCC00", "#FF6600", "#FFFFFF"];
const WIDTHS = [2, 3, 5, 8];

export default function AnnotationEditor({ imagePath, onClose }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const imgRef = useRef<HTMLImageElement | null>(null);
  const [objects, setObjects] = useState<AnnotationObj[]>([]);
  const [redoStack, setRedoStack] = useState<AnnotationObj[]>([]);
  const [tool, setTool] = useState<AnnotationTool>("rect");
  const [color, setColor] = useState("#FF0000");
  const [strokeWidth, setStrokeWidth] = useState(3);
  const [drawing, setDrawing] = useState(false);
  const [current, setCurrent] = useState<AnnotationObj | null>(null);
  const [textInput, setTextInput] = useState<{ x: number; y: number; visible: boolean }>({
    x: 0,
    y: 0,
    visible: false,
  });
  const [textValue, setTextValue] = useState("");
  const [saving, setSaving] = useState(false);
  const [ocrResult, setOcrResult] = useState<OcrResult | null>(null);
  const [ocrLoading, setOcrLoading] = useState(false);
  const [aiPanelOpen, setAiPanelOpen] = useState(false);
  const [aiPrompt, setAiPrompt] = useState(AI_TEMPLATES[0].prompt);
  const [aiResponse, setAiResponse] = useState<AiResponse | null>(null);
  const [aiLoading, setAiLoading] = useState(false);
  const [panelError, setPanelError] = useState("");
  const [panelCopied, setPanelCopied] = useState(false);

  useEffect(() => {
    invoke<number[]>("read_file_bytes", { path: imagePath }).then((bytes) => {
      const arr = new Uint8Array(bytes);
      const blob = new Blob([arr], { type: "image/png" });
      const url = URL.createObjectURL(blob);
      const img = new Image();
      img.onload = () => {
        imgRef.current = img;
        const canvas = canvasRef.current;
        if (canvas) {
          canvas.width = img.naturalWidth;
          canvas.height = img.naturalHeight;
          render();
        }
      };
      img.src = url;
    });
  }, [imagePath]);

  const render = useCallback(() => {
    const canvas = canvasRef.current;
    const img = imgRef.current;
    if (!canvas || !img) return;
    const ctx = canvas.getContext("2d")!;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, 0, 0);

    const allObjs = current ? [...objects, current] : objects;
    for (const obj of allObjs) {
      drawObject(ctx, obj);
    }
  }, [objects, current]);

  useEffect(() => {
    render();
  }, [render]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "z") {
        e.preventDefault();
        undo();
      } else if (e.ctrlKey && e.key === "y") {
        e.preventDefault();
        redo();
      } else if (e.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [objects, redoStack]);

  function drawObject(ctx: CanvasRenderingContext2D, obj: AnnotationObj) {
    ctx.strokeStyle = obj.color;
    ctx.fillStyle = obj.color;
    ctx.lineWidth = obj.width;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    switch (obj.tool) {
      case "rect": {
        if (obj.points.length < 2) break;
        const [p1, p2] = obj.points;
        ctx.strokeRect(p1.x, p1.y, p2.x - p1.x, p2.y - p1.y);
        break;
      }
      case "arrow": {
        if (obj.points.length < 2) break;
        const [start, end] = [obj.points[0], obj.points[obj.points.length - 1]];
        ctx.beginPath();
        ctx.moveTo(start.x, start.y);
        ctx.lineTo(end.x, end.y);
        ctx.stroke();
        const angle = Math.atan2(end.y - start.y, end.x - start.x);
        const headLen = 12 + obj.width * 2;
        ctx.beginPath();
        ctx.moveTo(end.x, end.y);
        ctx.lineTo(
          end.x - headLen * Math.cos(angle - Math.PI / 6),
          end.y - headLen * Math.sin(angle - Math.PI / 6)
        );
        ctx.moveTo(end.x, end.y);
        ctx.lineTo(
          end.x - headLen * Math.cos(angle + Math.PI / 6),
          end.y - headLen * Math.sin(angle + Math.PI / 6)
        );
        ctx.stroke();
        break;
      }
      case "pen": {
        if (obj.points.length < 2) break;
        ctx.beginPath();
        ctx.moveTo(obj.points[0].x, obj.points[0].y);
        for (let i = 1; i < obj.points.length; i++) {
          ctx.lineTo(obj.points[i].x, obj.points[i].y);
        }
        ctx.stroke();
        break;
      }
      case "highlighter": {
        if (obj.points.length < 2) break;
        ctx.save();
        ctx.globalAlpha = 0.35;
        ctx.lineWidth = obj.width * 3;
        ctx.beginPath();
        ctx.moveTo(obj.points[0].x, obj.points[0].y);
        for (let i = 1; i < obj.points.length; i++) {
          ctx.lineTo(obj.points[i].x, obj.points[i].y);
        }
        ctx.stroke();
        ctx.restore();
        break;
      }
      case "text": {
        if (!obj.text) break;
        ctx.font = `${obj.width * 6}px "Microsoft YaHei", sans-serif`;
        ctx.fillText(obj.text, obj.points[0].x, obj.points[0].y);
        break;
      }
    }
  }

  function getPos(e: React.MouseEvent): Point {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: (e.clientY - rect.top) * scaleY,
    };
  }

  function onMouseDown(e: React.MouseEvent) {
    if (tool === "text") {
      setTextInput({ x: e.clientX, y: e.clientY, visible: true });
      setTextValue("");
      (document.getElementById("text-input") as HTMLInputElement)?.focus();
      return;
    }
    const pos = getPos(e);
    setDrawing(true);
    setCurrent({ tool, points: [pos], color, width: strokeWidth });
  }

  function onMouseMove(e: React.MouseEvent) {
    if (!drawing || !current) return;
    const pos = getPos(e);
    if (tool === "pen" || tool === "highlighter") {
      setCurrent({ ...current, points: [...current.points, pos] });
    } else {
      setCurrent({ ...current, points: [current.points[0], pos] });
    }
  }

  function onMouseUp() {
    if (!drawing || !current) return;
    setDrawing(false);
    if (current.points.length >= 2) {
      setObjects((prev) => [...prev, current]);
      setRedoStack([]);
    }
    setCurrent(null);
  }

  function commitText() {
    if (textValue.trim()) {
      const canvas = canvasRef.current!;
      const rect = canvas.getBoundingClientRect();
      const scaleX = canvas.width / rect.width;
      const scaleY = canvas.height / rect.height;
      const x = (textInput.x - rect.left) * scaleX;
      const y = (textInput.y - rect.top) * scaleY;
      const obj: AnnotationObj = {
        tool: "text",
        points: [{ x, y }],
        color,
        width: strokeWidth,
        text: textValue,
      };
      setObjects((prev) => [...prev, obj]);
      setRedoStack([]);
    }
    setTextInput({ x: 0, y: 0, visible: false });
    setTextValue("");
  }

  function undo() {
    setObjects((prev) => {
      if (prev.length === 0) return prev;
      const last = prev[prev.length - 1];
      setRedoStack((r) => [...r, last]);
      return prev.slice(0, -1);
    });
  }

  function redo() {
    setRedoStack((prev) => {
      if (prev.length === 0) return prev;
      const last = prev[prev.length - 1];
      setObjects((o) => [...o, last]);
      return prev.slice(0, -1);
    });
  }

  async function save() {
    setSaving(true);
    try {
      const canvas = canvasRef.current!;
      const blob = await new Promise<Blob>((r) => canvas.toBlob((b) => r(b!), "image/png"));
      const buf = new Uint8Array(await blob.arrayBuffer());
      await invoke("save_annotated_image", { path: imagePath, data: Array.from(buf) });
      onClose();
    } finally {
      setSaving(false);
    }
  }

  async function copy() {
    setSaving(true);
    try {
      const canvas = canvasRef.current!;
      const blob = await new Promise<Blob>((r) => canvas.toBlob((b) => r(b!), "image/png"));
      const buf = new Uint8Array(await blob.arrayBuffer());
      await invoke("save_annotated_image", { path: imagePath, data: Array.from(buf) });
      await invoke("copy_image_to_clipboard", { path: imagePath });
      onClose();
    } finally {
      setSaving(false);
    }
  }

  async function flushCanvasToDisk() {
    const canvas = canvasRef.current!;
    const blob = await new Promise<Blob>((r) => canvas.toBlob((b) => r(b!), "image/png"));
    const buf = new Uint8Array(await blob.arrayBuffer());
    await invoke("save_annotated_image", { path: imagePath, data: Array.from(buf) });
  }

  async function runOcr() {
    setOcrLoading(true);
    setPanelError("");
    setOcrResult(null);
    setAiPanelOpen(true);
    try {
      await flushCanvasToDisk();
      const json = await invoke<string>("run_ocr", { imagePath });
      setOcrResult(JSON.parse(json));
      setAiResponse(null);
    } catch (e) {
      setPanelError(String(e));
    } finally {
      setOcrLoading(false);
    }
  }

  async function runAi() {
    setAiLoading(true);
    setPanelError("");
    setAiResponse(null);
    try {
      await flushCanvasToDisk();
      const json = await invoke<string>("analyze_image", { imagePath, prompt: aiPrompt });
      setAiResponse(JSON.parse(json));
      setOcrResult(null);
    } catch (e) {
      setPanelError(String(e));
    } finally {
      setAiLoading(false);
    }
  }

  async function copyPanelText() {
    const text = ocrResult?.text || aiResponse?.content;
    if (!text) return;
    await navigator.clipboard.writeText(text);
    setPanelCopied(true);
    setTimeout(() => setPanelCopied(false), 2000);
  }

  return (
    <div className="fixed inset-0 z-50 bg-black/90 flex flex-col">
      {/* Toolbar */}
      <div className="flex items-center gap-3 px-4 py-2 bg-gray-900 border-b border-gray-700 flex-wrap">
        <div className="flex gap-1">
          {TOOLS.map((t) => (
            <button
              key={t.id}
              onClick={() => setTool(t.id)}
              title={t.title}
              className={`p-2 rounded ${
                tool === t.id
                  ? "bg-primary text-white"
                  : "bg-gray-700 text-gray-300 hover:bg-gray-600"
              }`}
            >
              {t.icon}
            </button>
          ))}
        </div>

        <div className="w-px h-6 bg-gray-600" />

        <div className="flex gap-1 items-center">
          {COLORS.map((c) => (
            <button
              key={c}
              onClick={() => setColor(c)}
              className={`w-6 h-6 rounded-full border-2 ${
                color === c ? "border-white scale-110" : "border-gray-500"
              }`}
              style={{ backgroundColor: c }}
            />
          ))}
        </div>

        <div className="w-px h-6 bg-gray-600" />

        <div className="flex gap-1 items-center">
          {WIDTHS.map((w) => (
            <button
              key={w}
              onClick={() => setStrokeWidth(w)}
              className={`px-2 py-1 text-xs rounded ${
                strokeWidth === w
                  ? "bg-primary text-white"
                  : "bg-gray-700 text-gray-300 hover:bg-gray-600"
              }`}
            >
              {w}px
            </button>
          ))}
        </div>

        <div className="w-px h-6 bg-gray-600" />

        <div className="flex gap-1">
          <button
            onClick={undo}
            disabled={objects.length === 0}
            title="撤销 (Ctrl+Z)"
            className="p-2 bg-gray-700 text-gray-300 rounded hover:bg-gray-600 disabled:opacity-40"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M3 7 L7 3 M3 7 L7 11 M3 7 H11 C13 7 14 9 13 11" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
          <button
            onClick={redo}
            disabled={redoStack.length === 0}
            title="重做 (Ctrl+Y)"
            className="p-2 bg-gray-700 text-gray-300 rounded hover:bg-gray-600 disabled:opacity-40"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M13 7 L9 3 M13 7 L9 11 M13 7 H5 C3 7 2 9 3 11" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </button>
        </div>

        <div className="w-px h-6 bg-gray-600" />

        <div className="flex gap-1">
          <button
            onClick={runOcr}
            disabled={ocrLoading}
            title="OCR 文字识别"
            className="p-2 bg-purple-600 text-white rounded hover:bg-purple-500 disabled:opacity-50"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M2 5 V3 C2 2 2 2 3 2 H5 M11 2 H13 C14 2 14 2 14 3 V5 M14 11 V13 C14 14 14 14 13 14 H11 M5 14 H3 C2 14 2 14 2 13 V11" strokeLinecap="round" />
              <line x1="4" y1="6" x2="12" y2="6" strokeLinecap="round" />
              <line x1="4" y1="8" x2="12" y2="8" strokeLinecap="round" />
              <line x1="4" y1="10" x2="9" y2="10" strokeLinecap="round" />
            </svg>
          </button>
          <button
            onClick={() => {
              setAiPanelOpen(true);
              setOcrResult(null);
              setPanelError("");
            }}
            title="AI 分析"
            className="p-2 bg-indigo-600 text-white rounded hover:bg-indigo-500"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M8 1.5 L9.3 5.7 L13.5 7 L9.3 8.3 L8 12.5 L6.7 8.3 L2.5 7 L6.7 5.7 Z" strokeLinejoin="round" />
              <path d="M12.5 11 L13 12.5 L14.5 13 L13 13.5 L12.5 15 L12 13.5 L10.5 13 L12 12.5 Z" strokeLinejoin="round" />
            </svg>
          </button>
        </div>

        <div className="ml-auto flex gap-2">
          <button
            onClick={copy}
            disabled={saving}
            title="复制到剪贴板"
            className="p-2 bg-green-600 text-white rounded hover:bg-green-500 disabled:opacity-50"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
              <rect x="5" y="5" width="9" height="9" rx="1" />
              <path d="M11 3 H3 C2 3 2 3 2 4 V11" strokeLinecap="round" />
            </svg>
          </button>
          <button
            onClick={save}
            disabled={saving}
            title="保存"
            className="p-2 bg-primary text-white rounded hover:bg-primary/90 disabled:opacity-50"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M3 2 H11 L14 5 V13 C14 14 14 14 13 14 H3 C2 14 2 14 2 13 V3 C2 2 2 2 3 2 Z" />
              <rect x="5" y="2" width="6" height="4" />
              <rect x="5" y="10" width="6" height="4" />
            </svg>
          </button>
          <button
            onClick={onClose}
            title="关闭"
            className="p-2 bg-gray-600 text-white rounded hover:bg-gray-500"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
              <line x1="3" y1="3" x2="13" y2="13" strokeLinecap="round" />
              <line x1="13" y1="3" x2="3" y2="13" strokeLinecap="round" />
            </svg>
          </button>
        </div>
      </div>

      {/* Canvas area + side panel */}
      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 flex items-center justify-center overflow-auto p-4">
          <canvas
            ref={canvasRef}
            onMouseDown={onMouseDown}
            onMouseMove={onMouseMove}
            onMouseUp={onMouseUp}
            onMouseLeave={onMouseUp}
            className="max-w-full max-h-full cursor-crosshair shadow-2xl"
          />
        </div>

        {aiPanelOpen && (
          <div className="w-96 bg-gray-900 border-l border-gray-700 flex flex-col">
            <div className="flex items-center justify-between px-4 py-2 border-b border-gray-700">
              <span className="text-sm font-medium text-gray-200">
                {ocrLoading || ocrResult ? "OCR 识别结果" : "AI 分析"}
              </span>
              <button
                onClick={() => {
                  setAiPanelOpen(false);
                  setOcrResult(null);
                  setAiResponse(null);
                  setPanelError("");
                }}
                className="p-1 text-gray-400 hover:text-white"
                title="关闭面板"
              >
                <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
                  <line x1="3" y1="3" x2="13" y2="13" strokeLinecap="round" />
                  <line x1="13" y1="3" x2="3" y2="13" strokeLinecap="round" />
                </svg>
              </button>
            </div>

            <div className="flex-1 overflow-auto p-4 space-y-3">
              {panelError && (
                <div className="p-3 bg-red-900/30 border border-red-800 rounded text-red-400 text-xs break-all">
                  {panelError}
                </div>
              )}

              {ocrLoading && <p className="text-sm text-gray-400">识别中...</p>}

              {ocrResult && (
                <>
                  <div className="flex items-center justify-between text-xs text-gray-400">
                    <span>耗时 {ocrResult.duration_ms}ms</span>
                    <button
                      onClick={copyPanelText}
                      className="px-2 py-1 bg-gray-700 rounded hover:bg-gray-600 text-gray-200"
                    >
                      {panelCopied ? "已复制" : "复制文本"}
                    </button>
                  </div>
                  <div className="p-3 bg-gray-800 rounded border border-gray-700">
                    <pre className="whitespace-pre-wrap text-sm text-gray-100 font-mono">
                      {ocrResult.text || "(未识别到文字)"}
                    </pre>
                  </div>
                </>
              )}

              {!ocrLoading && !ocrResult && (
                <>
                  <div className="flex gap-1 flex-wrap">
                    {AI_TEMPLATES.map((t) => (
                      <button
                        key={t.id}
                        onClick={() => setAiPrompt(t.prompt)}
                        className={`px-2 py-1 text-xs rounded border ${
                          aiPrompt === t.prompt
                            ? "border-indigo-400 text-indigo-300 bg-indigo-900/30"
                            : "border-gray-600 text-gray-300 hover:border-indigo-500/50"
                        }`}
                      >
                        {t.label}
                      </button>
                    ))}
                  </div>

                  <textarea
                    value={aiPrompt}
                    onChange={(e) => setAiPrompt(e.target.value)}
                    rows={3}
                    className="w-full px-2 py-1.5 text-sm bg-gray-800 border border-gray-600 rounded text-gray-100 resize-none"
                    placeholder="输入提示词..."
                  />

                  <button
                    onClick={runAi}
                    disabled={aiLoading || !aiPrompt.trim()}
                    className="w-full px-3 py-2 text-sm bg-indigo-600 text-white rounded hover:bg-indigo-500 disabled:opacity-50"
                  >
                    {aiLoading ? "分析中..." : "开始分析"}
                  </button>

                  {aiResponse && (
                    <>
                      <div className="flex items-center justify-between text-xs text-gray-400">
                        <span>
                          {aiResponse.model} · {aiResponse.duration_ms}ms
                        </span>
                        <button
                          onClick={copyPanelText}
                          className="px-2 py-1 bg-gray-700 rounded hover:bg-gray-600 text-gray-200"
                        >
                          {panelCopied ? "已复制" : "复制"}
                        </button>
                      </div>
                      <div className="p-3 bg-gray-800 rounded border border-gray-700">
                        <div className="whitespace-pre-wrap text-sm text-gray-100">
                          {aiResponse.content}
                        </div>
                      </div>
                    </>
                  )}
                </>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Text input overlay */}
      {textInput.visible && (
        <input
          id="text-input"
          type="text"
          value={textValue}
          onChange={(e) => setTextValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commitText();
            if (e.key === "Escape") setTextInput({ x: 0, y: 0, visible: false });
          }}
          onBlur={commitText}
          className="fixed z-50 px-2 py-1 text-sm bg-white text-black border-2 border-primary rounded shadow-lg"
          style={{ left: textInput.x, top: textInput.y }}
          placeholder="输入文字..."
          autoFocus
        />
      )}
    </div>
  );
}
