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

interface OcrResult {
  text: string;
  language: string;
  confidence: number;
  duration_ms: number;
}

/// Editor windows are created per capture with their geometry in the query
/// string (physical pixels): path, window rect (x/y), image rect (w/h) and
/// the image offset inside the window (ox/oy) — the window keeps a
/// transparent margin around the image for the outer glow.
function editorQuery() {
  const p = new URLSearchParams(window.location.search);
  return {
    path: p.get("path") ?? "",
    x: Number(p.get("x") ?? 0),
    y: Number(p.get("y") ?? 0),
    w: Number(p.get("w") ?? 0),
    h: Number(p.get("h") ?? 0),
    ox: Number(p.get("ox") ?? 0),
    oy: Number(p.get("oy") ?? 0),
  };
}

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

/// Approximate rendered width of the icon toolbar in CSS pixels; captures
/// narrower than this still get a window wide enough to show it in full.
const TOOLBAR_W_CSS = 585;

export default function AnnotationEditor() {
  const { path: imagePath, x: winX, y: winY, w: frameW, h: frameH, ox: padX, oy: padY } =
    editorQuery();
  const onClose = useCallback(() => {
    invoke("close_editor").catch(() => {});
  }, []);
  const toolbarRef = useRef<HTMLDivElement>(null);
  const [toolbarW, setToolbarW] = useState(TOOLBAR_W_CSS);

  // Measure the real rendered toolbar width so the right-edge alignment and
  // never-clip logic below use exact numbers instead of estimates.
  useEffect(() => {
    const el = toolbarRef.current;
    if (!el) return;
    const measure = () => setToolbarW(el.offsetWidth);
    measure();
    const raf = requestAnimationFrame(measure);
    return () => cancelAnimationFrame(raf);
  }, []);
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
  const [panelOpen, setPanelOpen] = useState(false);
  const [panelError, setPanelError] = useState("");
  const [panelCopied, setPanelCopied] = useState(false);
  const [widthMenuOpen, setWidthMenuOpen] = useState(false);

  // Displayed image size and in-window offset in CSS pixels (query params
  // are physical pixels).
  const [geo] = useState(() => {
    const dpr = window.devicePixelRatio || 1;
    return {
      disp: {
        w: Math.max(1, Math.round(frameW / dpr)),
        h: Math.max(1, Math.round(frameH / dpr)),
      },
      off: { x: Math.round(padX / dpr), y: Math.round(padY / dpr) },
    };
  });
  const { disp, off } = geo;

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
    if (!widthMenuOpen) return;
    const close = () => setWidthMenuOpen(false);
    document.addEventListener("click", close);
    return () => document.removeEventListener("click", close);
  }, [widthMenuOpen]);

  function onToolbarMouseDown(e: React.MouseEvent) {
    if ((e.target as HTMLElement).closest("button")) return;
    e.preventDefault();
    invoke("start_window_drag").catch(() => {});
  }

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "z") {
        e.preventDefault();
        undo();
      } else if (e.ctrlKey && e.key === "y") {
        e.preventDefault();
        redo();
      } else if (e.key === "Escape") {
        if (panelOpen) {
          setPanelOpen(false);
        } else {
          onClose();
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [objects, redoStack, panelOpen]);

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

  async function extractText() {
    setOcrLoading(true);
    setPanelError("");
    setOcrResult(null);
    setPanelOpen(true);
    try {
      await flushCanvasToDisk();
      const json = await invoke<string>("run_ocr", { imagePath });
      setOcrResult(JSON.parse(json));
    } catch (e) {
      setPanelError(String(e));
    } finally {
      setOcrLoading(false);
    }
  }

  async function pinImage() {
    setSaving(true);
    try {
      // Bake annotations into the file first, then pin the image at its
      // exact on-screen position; the editor window goes away afterwards.
      await flushCanvasToDisk();
      const dpr = window.devicePixelRatio || 1;
      await invoke("pin_image", {
        path: imagePath,
        x: Math.round(winX + off.x * dpr),
        y: Math.round(winY + off.y * dpr),
        width: Math.round(disp.w * dpr),
        height: Math.round(disp.h * dpr),
      });
      onClose();
    } catch (e) {
      setPanelError(String(e));
      setPanelOpen(true);
    } finally {
      setSaving(false);
    }
  }

  async function copyPanelText() {
    if (!ocrResult?.text) return;
    await navigator.clipboard.writeText(ocrResult.text);
    setPanelCopied(true);
    setTimeout(() => setPanelCopied(false), 2000);
  }

  return (
    <div className="w-screen h-screen overflow-hidden">
      {/* Image at its exact on-screen position; the transparent margin around
          it hosts the outer glow. The glow border is an overlay div (an inset
          shadow on the canvas itself would be hidden beneath the bitmap). */}
      <div
        className="absolute"
        style={{ left: off.x, top: off.y, width: disp.w, height: disp.h }}
      >
        <div className="relative" style={{ width: disp.w, height: disp.h }}>
          <canvas
            ref={canvasRef}
            onMouseDown={onMouseDown}
            onMouseMove={onMouseMove}
            onMouseUp={onMouseUp}
            onMouseLeave={onMouseUp}
            style={{ width: disp.w, height: disp.h }}
            className="block cursor-crosshair"
          />
          <div className="pointer-events-none absolute -inset-px border border-[#00AEFF] shadow-[0_0_14px_rgba(0,174,255,0.5)]" />
        </div>
      </div>

      {/* Toolbar strip below the image (transparent background); the bar's
          right edge hugs the image's right edge when the image is wide enough,
          otherwise it right-aligns to the window so it is never clipped. */}
      <div
        className="absolute left-0 right-0 flex justify-end items-start"
        style={{ top: off.y + disp.h, bottom: 0 }}
      >
        <div
          ref={toolbarRef}
          className="relative"
          style={{
            marginRight: Math.max(
              0,
              window.innerWidth - Math.max(off.x + disp.w, toolbarW + 4)
            ),
          }}
        >
            <div
              onMouseDown={onToolbarMouseDown}
              className="mt-2 mr-1 flex items-center gap-1 px-2 py-2 rounded-lg bg-gray-900 border border-gray-700 shadow-xl"
            >
              <div className="flex gap-1">
                {TOOLS.map((t) => (
                  <button
                    key={t.id}
                    onClick={() => setTool(t.id)}
                    title={t.title}
                    className={`p-1.5 rounded ${
                      tool === t.id
                        ? "text-primary bg-primary/10"
                        : "text-gray-400 hover:text-white hover:bg-gray-800"
                    }`}
                  >
                    {t.icon}
                  </button>
                ))}
              </div>

              <div className="w-px h-6 bg-gray-700" />

              <div className="flex gap-1 items-center">
                {COLORS.map((c) => (
                  <button
                    key={c}
                    onClick={() => setColor(c)}
                    title={c}
                    className={`w-5 h-5 rounded-full border-2 ${
                      color === c ? "border-white scale-110" : "border-gray-500"
                    }`}
                    style={{ backgroundColor: c }}
                  />
                ))}
              </div>

              <div className="w-px h-6 bg-gray-700" />

              <div className="relative">
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setWidthMenuOpen((o) => !o);
                  }}
                  title="线条粗细"
                  className="flex items-center gap-1 px-1.5 py-1.5 rounded text-gray-400 hover:text-white hover:bg-gray-800"
                >
                  <div className="w-7 rounded-full bg-current" style={{ height: strokeWidth }} />
                  <svg width="8" height="8" viewBox="0 0 8 8" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="M1 3 L4 6 L7 3" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                </button>
                {widthMenuOpen && (
                  <div className="absolute bottom-full left-0 mb-1 z-50 bg-gray-900 border border-gray-700 rounded shadow-lg py-1">
                    {WIDTHS.map((w) => (
                      <button
                        key={w}
                        onClick={(e) => {
                          e.stopPropagation();
                          setStrokeWidth(w);
                          setWidthMenuOpen(false);
                        }}
                        className={`flex items-center justify-center w-16 h-8 hover:bg-gray-700 ${
                          strokeWidth === w ? "text-primary" : "text-gray-300 hover:text-white"
                        }`}
                      >
                        <div className="w-10 rounded-full bg-current" style={{ height: w }} />
                      </button>
                    ))}
                  </div>
                )}
              </div>

              <div className="w-px h-6 bg-gray-700" />

              <div className="flex gap-1">
                <button
                  onClick={undo}
                  disabled={objects.length === 0}
                  title="撤销 (Ctrl+Z)"
                  className="p-1.5 rounded text-gray-400 hover:text-white hover:bg-gray-800 disabled:opacity-30 disabled:hover:bg-transparent"
                >
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="M3 7 L7 3 M3 7 L7 11 M3 7 H11 C13 7 14 9 13 11" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                </button>
                <button
                  onClick={redo}
                  disabled={redoStack.length === 0}
                  title="重做 (Ctrl+Y)"
                  className="p-1.5 rounded text-gray-400 hover:text-white hover:bg-gray-800 disabled:opacity-30 disabled:hover:bg-transparent"
                >
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="M13 7 L9 3 M13 7 L9 11 M13 7 H5 C3 7 2 9 3 11" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                </button>
              </div>

              <div className="w-px h-6 bg-gray-700" />

              <div className="flex gap-1">
                <button
                  onClick={pinImage}
                  disabled={saving}
                  title="贴图（双击贴图关闭）"
                  className="p-1.5 rounded text-amber-400 hover:text-amber-300 hover:bg-gray-800 disabled:opacity-30 disabled:hover:bg-transparent"
                >
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M12 17v5" />
                    <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1z" />
                  </svg>
                </button>
                <button
                  onClick={extractText}
                  disabled={ocrLoading}
                  title="提取文本"
                  className="p-1.5 rounded text-purple-400 hover:text-purple-300 hover:bg-gray-800 disabled:opacity-30 disabled:hover:bg-transparent"
                >
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="M2 5 V3 C2 2 2 2 3 2 H5 M11 2 H13 C14 2 14 2 14 3 V5 M14 11 V13 C14 14 14 14 13 14 H11 M5 14 H3 C2 14 2 14 2 13 V11" strokeLinecap="round" />
                    <line x1="4" y1="6" x2="12" y2="6" strokeLinecap="round" />
                    <line x1="4" y1="8" x2="12" y2="8" strokeLinecap="round" />
                    <line x1="4" y1="10" x2="9" y2="10" strokeLinecap="round" />
                  </svg>
                </button>
              </div>

              <div className="w-px h-6 bg-gray-700" />

              <div className="flex gap-1">
                <button
                  onClick={copy}
                  disabled={saving}
                  title="复制到剪贴板"
                  className="p-1.5 rounded text-green-400 hover:text-green-300 hover:bg-gray-800 disabled:opacity-30 disabled:hover:bg-transparent"
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
                  className="p-1.5 rounded text-primary hover:text-primary/80 hover:bg-gray-800 disabled:opacity-30 disabled:hover:bg-transparent"
                >
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <path d="M3 2 H11 L14 5 V13 C14 14 14 14 13 14 H3 C2 14 2 14 2 13 V3 C2 2 2 2 3 2 Z" />
                    <rect x="5" y="2" width="6" height="4" />
                    <rect x="5" y="10" width="6" height="4" />
                  </svg>
                </button>
                <button
                  onClick={onClose}
                  title="关闭 (Esc)"
                  className="p-1.5 rounded text-gray-400 hover:text-red-400 hover:bg-gray-800"
                >
                  <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <line x1="3" y1="3" x2="13" y2="13" strokeLinecap="round" />
                    <line x1="13" y1="3" x2="3" y2="13" strokeLinecap="round" />
                  </svg>
                </button>
              </div>
            </div>

            {/* Extracted text panel, floating above the toolbar */}
            {panelOpen && (
              <div className="absolute bottom-full right-0 mb-2 w-[min(440px,calc(100vw-16px))] max-h-[55vh] flex flex-col bg-gray-900 border border-gray-700 rounded-lg shadow-2xl overflow-hidden">
                <div className="flex items-center justify-between px-3 py-2 border-b border-gray-700 shrink-0">
                  <span className="text-sm font-medium text-gray-200">提取的文本</span>
                  <div className="flex items-center gap-2">
                    {ocrResult && !ocrLoading && (
                      <button
                        onClick={copyPanelText}
                        className="px-2 py-0.5 text-xs bg-gray-700 rounded hover:bg-gray-600 text-gray-200"
                      >
                        {panelCopied ? "已复制" : "复制"}
                      </button>
                    )}
                    <button
                      onClick={() => setPanelOpen(false)}
                      title="关闭面板"
                      className="p-1 text-gray-400 hover:text-white"
                    >
                      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
                        <line x1="3" y1="3" x2="13" y2="13" strokeLinecap="round" />
                        <line x1="13" y1="3" x2="3" y2="13" strokeLinecap="round" />
                      </svg>
                    </button>
                  </div>
                </div>
                <div className="flex-1 overflow-auto p-3">
                  {panelError && (
                    <div className="p-2 bg-red-900/30 border border-red-800 rounded text-red-400 text-xs break-all">
                      {panelError}
                    </div>
                  )}
                  {ocrLoading && <p className="text-sm text-gray-400">识别中...</p>}
                  {ocrResult && !ocrLoading && (
                    <pre className="whitespace-pre-wrap text-sm text-gray-100 font-mono">
                      {ocrResult.text || "(未识别到文字)"}
                    </pre>
                  )}
                </div>
              </div>
            )}
        </div>
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
