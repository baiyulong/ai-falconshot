# AGENTS.md — AI 编码代理指引

本文件供 AI 编码代理（Claude Code、Codex、ZCode 等）在本仓库工作时参考。

## 项目概要

FalconShot：Windows 智能截图工具（截图 → 标注 → 贴图 / OCR / AI 识别）。Rust workspace + Tauri 2 + React 19。目标平台 Windows 10/11，产品与 UI 文案为中文。

## 仓库结构

```text
crates/            平台无关的领域层（每个能力一个 crate）
  capture-core     截图抽象（CaptureBackend trait、Rect、CaptureFrame）
  overlay-core     选区模型
  floating-core    贴图状态（FloatingState/FloatingWindow trait）
  annotation-core  标注对象模型
  image-core       图像处理（取色、特效）
  clipboard-core   剪贴板抽象（ClipboardBackend trait）
  hotkey-core      快捷键抽象
  ocr-core         OCR 抽象（WindowsOcrProvider = Windows.Media.Ocr）
  ai-core          AI 调用抽象（当前 app 未使用，命令在 app 层实现）
  privacy-core     敏感信息检测
  storage-core     历史记录 JSON 存储
  settings-core    应用设置（%APPDATA%/FalconShot/settings.json）
platform/windows   Win32 原生实现（capture/overlay/floating/clipboard/hotkey）
apps/desktop-tauri Tauri 2 应用
  src/             React 前端（App.tsx 主界面、AnnotationEditor.tsx 编辑器）
  src-tauri/       Rust 命令层（commands/*.rs）
.github/workflows  ci.yml（检查）、release.yml（打 v* 标签出 MSI）
```

## 构建与检查（提交前全部通过）

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings   # 0 警告，CI 同配置
cargo test --workspace
cd apps/desktop-tauri && npx tsc --noEmit && npm run build
```

本地运行：`cd apps/desktop-tauri && npm run tauri dev`（dev 模式主窗口自动显示）。

## 关键约定与陷阱

- **坐标系**：全链路使用物理像素（虚拟屏幕坐标，原点在主屏左上）。遮罩窗口返回的选区、贴图/编辑器窗口定位均为物理像素；前端 CSS 尺寸 = 物理值 ÷ `devicePixelRatio`。GDI 的 `GetSystemMetrics` 在 PerMonitorV2 进程中返回物理值。
- **GDI 字节序是 BGRA**：把 `RgbaImage` 上传给 DIB/位图前必须交换 R/B（见 overlay.rs `upload_bitmap`），否则预览红蓝颠倒。`UpdateLayeredWindow` 需要**预乘 alpha**。
- **debug 构建整数溢出会 panic**：所有 u8 像素运算先转 u32（历史教训：`dim_image`、`decorate_pin` 都踩过）。
- **线程模型**：遮罩（overlay）在 `spawn_blocking` 线程上创建并自持消息循环；贴图/编辑器窗口在 **Tauri 主线程**创建（`run_on_main_thread`），依赖 tao 事件循环派发消息。floating 的 `wnd_proc` **绝不能 `PostQuitMessage`**（会退出整个应用）；关闭贴图用 `DestroyWindow`（双击触发）。
- **编辑器窗口**：每次截图新建独立无边框透明窗口（`open_editor_window`），几何通过 URL query 传给前端（物理像素：x/y 窗口位、w/h 图像尺寸、ox/oy 画布偏移）；窗口四周留 12 逻辑像素透明衬距给外侧光晕，clamp 到显示器范围（防 fullscreen panic）。不要移动/缩放已存在的窗口——WebView2 合成层不会跟随更新（历史 bug）。
- **Tauri 命令**：阻塞操作（抓屏、文件 IO）用 `spawn_blocking`；需要主线程的操作用 `run_on_main_thread`。新命令记得加进 `lib.rs` 的 `generate_handler!`。编辑器窗口的权限靠 capabilities 的 `editor-*` 模式。
- **settings**：`settings-core/types.rs` 是唯一 schema；新增字段必须 `#[serde(default)]`（兼容旧 settings.json）。保存后 `save_settings` 会重新应用全局热键。
- **前端**：React 19 + Tailwind；编辑器面板中文案为中文；`react-markdown` 用于渲染 AI 输出。画布标注导出用 `canvas.toBlob`（CSS 边框/光晕不会进入导出图）。
- **法务**：不复制任何第三方截图软件的源码、资源、图标与文案（见 FalconShot_PRD.md §17）。
