# LynxShot 专业版技术栈设计文档

> 面向 Windows 与 macOS 的专业级 AI 截图、贴图、OCR 与 AI 识图工具技术栈设计

**版本**：V0.1  
**文档状态**：技术方案草案  
**建议产品代号**：LynxShot / 灵猞截图  
**目标平台**：Windows、macOS  
**核心路线**：Rust Core + 平台原生能力 + Tauri/React 管理界面

---

## 1. 技术路线总览

### 1.1 总体原则

本产品定位为专业级截图工具，因此不建议采用“纯 WebView 截图工具”路线。截图遮罩、贴图窗口、鼠标穿透、窗口置顶、多屏 DPI、Retina、HDR、系统权限等能力都需要接近系统底层的控制能力。

推荐总体架构：

```text
Rust Core
+
Windows Native Layer / macOS Native Layer
+
Tauri 2 + React 管理界面
+
原生截图遮罩窗口
+
原生贴图悬浮窗口
```

### 1.2 核心分工

- **Rust Core**：跨平台核心能力，包括截图抽象、图像处理、OCR 调度、AI 调度、隐私安全、存储、配置、快捷键抽象。
- **Windows Native Layer**：负责 Windows 平台截图、贴图、窗口、DPI、剪贴板、全局热键等系统能力。
- **macOS Native Layer**：负责 macOS 平台 ScreenCaptureKit、AppKit 浮窗、Retina 坐标、Vision OCR、系统权限等能力。
- **Tauri 2 + React**：负责设置页、历史页、OCR 结果页、AI 对话页、模型配置页等复杂业务 UI。
- **原生窗口层**：负责截图遮罩、贴图窗口、鼠标穿透、置顶、透明、拖拽、缩放等关键交互。

---

## 2. 跨平台公共技术栈

### 2.1 公共技术选择

| 模块 | 技术选择 | 说明 |
|---|---|---|
| 核心语言 | Rust | 负责性能敏感、系统集成、图像处理、OCR/AI 调度、安全存储 |
| UI 语言 | TypeScript | 负责管理界面和复杂业务界面 |
| UI 框架 | React | 设置页、历史页、AI 对话、OCR 结果等界面开发效率高 |
| 样式框架 | Tailwind CSS | 快速实现现代化 UI |
| 桌面壳 | Tauri 2 | 用系统 WebView + Rust 后端构建轻量跨平台桌面应用 |
| 异步运行时 | Tokio | AI 请求、OCR 调度、文件 IO、后台任务 |
| 序列化 | serde / serde_json | 配置、历史、OCR/AI 结果序列化 |
| 日志 | tracing / tracing-subscriber | 结构化日志、排障、审计 |
| 存储 | SQLite | 历史记录、配置、OCR/AI 结果、贴图状态 |
| SQLite 访问 | sqlx 或 rusqlite | 根据团队偏好选择 |
| 安全存储 | keyring / 系统 Keychain / DPAPI | API Key、Token、许可证等敏感数据 |
| 加密 | aes-gcm / zeroize | 本地缓存加密、敏感内存清理 |
| 图像处理 | image / imageproc / fast_image_resize | 裁剪、缩放、旋转、模糊、马赛克、格式转换 |
| 可选 GPU | wgpu | 高性能渲染、跨平台图形抽象 |

---

## 3. Rust Workspace 结构建议

建议从项目初期就采用 Rust workspace，以便后续拆分平台实现、测试、复用和企业版扩展。

```text
lynxshot/
├── apps/
│   └── desktop-tauri/
│
├── crates/
│   ├── capture-core/
│   ├── overlay-core/
│   ├── floating-core/
│   ├── annotation-core/
│   ├── image-core/
│   ├── clipboard-core/
│   ├── hotkey-core/
│   ├── ocr-core/
│   ├── ai-core/
│   ├── privacy-core/
│   ├── storage-core/
│   ├── settings-core/
│   ├── license-core/
│   └── telemetry-core/
│
├── platform/
│   ├── windows/
│   └── macos/
│
└── docs/
```

### 3.1 核心 crate 说明

#### capture-core

负责平台无关的截图抽象。

```text
capture-core
├── CaptureSession
├── CaptureTarget
├── CaptureFrame
├── CaptureOptions
├── MonitorInfo
├── WindowInfo
├── CoordinateMapper
└── CaptureBackend trait
```

#### overlay-core

负责截图遮罩、选区、放大镜、工具栏等抽象。

```text
overlay-core
├── OverlayWindow trait
├── SelectionModel
├── Magnifier
├── ElementHighlighter
├── Toolbar
├── KeyboardController
└── PointerController
```

#### floating-core

负责贴图窗口抽象和状态管理。

```text
floating-core
├── FloatingWindow trait
├── FloatingImage
├── TransformState
├── OpacityState
├── PinState
├── MousePassthroughState
├── GroupState
└── WorkspaceState
```

#### annotation-core

负责标注对象模型与渲染数据结构。

```text
annotation-core
├── AnnotationDocument
├── AnnotationLayer
├── ShapeObject
├── TextObject
├── BlurObject
├── MosaicObject
├── HistoryStack
└── Renderer trait
```

#### image-core

负责图像处理。

```text
image-core
├── crop
├── resize
├── rotate
├── flip
├── blur
├── mosaic
├── encode
├── decode
└── ocr_preprocess
```

#### ocr-core

负责 OCR Provider 抽象与后处理。

```text
ocr-core
├── OcrProvider trait
├── OcrRequest
├── OcrResult
├── OcrBlock
├── OcrTable
├── OcrPostProcessor
└── OcrLanguage
```

#### ai-core

负责 AI 多模态模型抽象、Prompt 模板、多轮上下文。

```text
ai-core
├── VisionProvider trait
├── AiRequest
├── AiResponse
├── PromptTemplate
├── ConversationContext
├── ModelRouter
├── ImageCompressor
└── PrivacyGate
```

---

## 4. Windows 专业版技术栈

### 4.1 Windows 总体方案

Windows 平台建议作为第一优先级平台，先把专业截图软件最核心的体验打牢：截图遮罩、多屏 DPI、贴图置顶、鼠标穿透、透明浮窗、全局快捷键、剪贴板、OCR、AI。

推荐 Windows 技术栈：

```text
Rust
+
windows-rs
+
Windows Graphics Capture API
+
DXGI Desktop Duplication API fallback
+
Win32 Native Window
+
Tauri 2 / React 管理界面
```

### 4.2 Windows 截图引擎

| 能力 | 推荐技术 |
|---|---|
| 屏幕捕获 | Windows Graphics Capture API |
| 兼容性备用方案 | DXGI Desktop Duplication API |
| Rust 调 Windows API | windows-rs |
| 可评估封装库 | windows-capture |
| 窗口枚举 | Win32 API |
| 多显示器 | Win32 Display / Monitor API |
| DPI 感知 | Per-Monitor DPI Awareness V2 |
| 鼠标指针捕获 | Win32 Cursor API / Capture API options |
| HDR 处理 | Windows Graphics Capture / DXGI 色彩路径 |

#### Windows capture-platform 模块建议

```text
capture-platform-windows
├── WindowsGraphicsCaptureBackend
├── DxgiDuplicationBackend
├── Win32WindowEnumerator
├── MonitorEnumerator
├── DpiConverter
├── CursorCaptureController
├── HdrColorProcessor
└── CaptureExclusionManager
```

#### Windows 截图关键难点

- 多显示器负坐标。
- 125%、150%、175% 等 DPI 缩放。
- 截图坐标需要统一为物理像素或统一逻辑坐标，不能混用。
- HDR 屏幕截图颜色容易失真，需要 SDR/HDR 策略。
- 截图时需要避免捕获自身遮罩窗口。
- 被遮挡窗口截图可能需要不同策略。
- 远程桌面、虚拟机、显卡混合环境需要专项测试。
- UAC 安全桌面无法被普通权限进程捕获，需要明确产品边界。

### 4.3 Windows 截图遮罩窗口

专业版截图遮罩不建议使用普通 Tauri WebView。应使用原生 Win32 窗口实现，确保透明、置顶、鼠标事件、多屏、DPI 和截图排除可控。

| 能力 | 推荐技术 |
|---|---|
| 全屏遮罩 | Win32 无边框透明窗口 |
| 置顶 | WS_EX_TOPMOST |
| 半透明绘制 | Layered Window / Direct2D |
| 鼠标事件 | Win32 message loop |
| 多屏覆盖 | 每个显示器一个 overlay 或虚拟桌面大窗口 |
| 高性能绘制 | Direct2D / WGPU / Skia 可选 |

#### overlay-windows 模块建议

```text
overlay-windows
├── Win32OverlayWindow
├── SelectionRenderer
├── MagnifierRenderer
├── ToolbarHost
├── HitTestHandler
├── KeyboardHandler
└── DpiAwareGeometry
```

### 4.4 Windows 贴图悬浮窗口

贴图窗口是产品核心竞争力，必须使用原生窗口实现。

| 能力 | 推荐技术 |
|---|---|
| 置顶贴图 | Win32 Native Window |
| 透明度 | Layered Window |
| 鼠标穿透 | WS_EX_TRANSPARENT / hit-test 控制 |
| 无边框窗口 | WS_POPUP |
| 缩放旋转 | Direct2D / WGPU / CPU fallback |
| 多贴图管理 | Rust window manager |
| 状态恢复 | SQLite + 文件缓存 |

#### floating-window-windows 模块建议

```text
floating-window-windows
├── NativeFloatingWindow
├── ImageSurface
├── TransformController
├── OpacityController
├── HitTestController
├── AlwaysOnTopController
├── GroupManager
└── WorkspaceRestorer
```

#### Windows 贴图专业要求

- 支持几十个贴图窗口同时存在。
- 贴图窗口创建、隐藏、恢复要足够快。
- 鼠标穿透模式必须稳定。
- 透明度、旋转、缩放实时生效。
- 拖拽跨屏时不能发生 DPI 坐标跳变。
- 支持缩略图模式。
- 支持贴图分组。
- 支持工作区保存与崩溃恢复。

### 4.5 Windows OCR

| 场景 | 推荐技术 |
|---|---|
| 本地 OCR | Tesseract / Windows OCR API |
| 跨平台统一 OCR | Tesseract Provider |
| Rust 绑定 | tesseract / tesseract-rs |
| 云端 OCR | Azure AI Vision / Tencent OCR / OCR.space / Custom HTTP Provider |

#### Windows OCR Provider 设计

```text
ocr-windows
├── WindowsOcrProvider
├── TesseractProvider
├── CloudOcrProvider
└── OcrPostProcessor
```

### 4.6 Windows 打包与部署

| 能力 | 推荐技术 |
|---|---|
| 安装包 | MSI |
| 可选安装包 | MSIX |
| 自动更新 | Tauri updater 或自研更新器 |
| 签名 | Windows Code Signing Certificate |
| 企业部署 | MSI 静默安装 |
| 策略下发 | Registry / JSON Policy / Group Policy |
| 崩溃日志 | minidump + tracing |

---

## 5. macOS 专业版技术栈

### 5.1 macOS 总体方案

macOS 平台需要重点关注 Screen Recording 权限、Retina 坐标、Spaces/Mission Control、AppKit 窗口层级、Vision OCR 和签名公证。

推荐 macOS 技术栈：

```text
Rust
+
ScreenCaptureKit
+
AppKit
+
CoreGraphics
+
Vision
+
objc2 / Swift bridge
+
Tauri 2 / React 管理界面
```

### 5.2 macOS 截图引擎

| 能力 | 推荐技术 |
|---|---|
| 高性能屏幕捕获 | ScreenCaptureKit |
| 单帧截图 | SCScreenshotManager |
| 显示器/窗口/应用过滤 | SCShareableContent / SCContentFilter |
| Rust 绑定 | screencapturekit-rs 或 Swift bridge |
| 备用旧方案 | CoreGraphics CGWindowList / CGDisplay APIs |
| 权限 | Screen Recording Permission |
| HDR/SDR | ScreenCaptureKit / 系统截图策略 |

#### capture-platform-macos 模块建议

```text
capture-platform-macos
├── ScreenCaptureKitBackend
├── ScreenshotManagerBackend
├── CoreGraphicsFallbackBackend
├── ShareableContentEnumerator
├── ContentFilterBuilder
├── ScreenRecordingPermissionManager
├── DisplayCoordinateMapper
└── HdrCaptureProcessor
```

#### macOS 截图关键难点

- 首次使用需要 Screen Recording 权限。
- 授权后可能需要重启应用才能正常捕获。
- Retina 逻辑坐标与物理像素坐标需要统一映射。
- 多显示器 Retina / 非 Retina 混合场景容易出现坐标偏差。
- Spaces、Mission Control、全屏应用的窗口表现需要专项测试。
- 菜单栏、Dock、系统浮窗层级需要处理。
- HDR/SDR 截图需要产品策略。

### 5.3 macOS 截图遮罩窗口

macOS 截图遮罩建议使用 AppKit 原生 NSWindow 实现。

| 能力 | 推荐技术 |
|---|---|
| 原生窗口 | AppKit NSWindow |
| 透明窗口 | NSWindow + transparent background |
| 置顶层级 | NSWindow.Level / CoreGraphics Window Levels |
| 鼠标事件 | AppKit NSEvent / NSResponder |
| 绘制 | CoreGraphics / Metal / WGPU |
| Rust 调 AppKit | objc2 / objc2-app-kit 或 Swift bridge |

#### overlay-macos 模块建议

```text
overlay-macos
├── AppKitOverlayWindow
├── NSViewRenderer
├── SelectionRenderer
├── MagnifierRenderer
├── ToolbarHost
├── EventMonitor
└── RetinaCoordinateMapper
```

### 5.4 macOS 贴图悬浮窗口

macOS 贴图窗口建议使用 AppKit NSWindow，不建议使用 WebView 窗口作为贴图窗口。

| 能力 | 推荐技术 |
|---|---|
| 贴图窗口 | AppKit NSWindow |
| 透明背景 | isOpaque = false / clear background |
| 置顶 | NSWindow.Level / CoreGraphics window level |
| 鼠标穿透 | ignoresMouseEvents |
| 图像渲染 | CoreGraphics / Metal / WGPU |
| 多窗口管理 | Rust + AppKit bridge |
| 状态恢复 | SQLite + 文件缓存 |

#### floating-window-macos 模块建议

```text
floating-window-macos
├── NativeFloatingWindow
├── NSWindowBridge
├── ImageLayer
├── TransformController
├── OpacityController
├── MousePassthroughController
├── WindowLevelController
├── WorkspaceManager
└── DisplayMapper
```

#### macOS 贴图专业要求

- 与 Mission Control / Spaces 尽可能兼容。
- 多显示器拖拽时坐标稳定。
- 支持 Retina 缩放。
- 支持鼠标穿透。
- 支持窗口层级切换。
- 支持隐藏全部贴图。
- 支持工作区恢复。
- 支持贴图分组。

### 5.5 macOS OCR

| 场景 | 推荐技术 |
|---|---|
| 本地 OCR 第一选择 | Apple Vision Framework |
| Rust 调用方式 | Swift bridge / objc2 / FFI 小模块 |
| 跨平台统一 OCR | Tesseract Provider |
| 云端 OCR | Azure / Tencent / OCR.space / Custom HTTP Provider |

#### macOS OCR Provider 设计

```text
ocr-macos
├── AppleVisionOcrProvider
├── TesseractProvider
├── CloudOcrProvider
└── OcrPostProcessor
```

#### 建议

macOS 上优先使用 Apple Vision OCR，因为它是系统级、本地处理、隐私友好，适合截图 OCR。Tesseract 作为跨平台统一备用方案。

### 5.6 macOS 打包与部署

| 能力 | 推荐技术 |
|---|---|
| 安装包 | DMG |
| 企业部署 | PKG 可选 |
| 签名 | Apple Developer ID |
| 公证 | Apple Notarization |
| 自动更新 | Sparkle 或 Tauri updater |
| 权限说明 | Info.plist |
| 权限引导 | System Settings deep link |
| 崩溃日志 | macOS crash report + tracing |

---

## 6. Windows 与 macOS 技术栈对比

| 模块 | Windows 专业方案 | macOS 专业方案 |
|---|---|---|
| 截图 API | Windows Graphics Capture API + DXGI fallback | ScreenCaptureKit + CoreGraphics fallback |
| Rust 系统绑定 | windows-rs | objc2 / Swift bridge / screencapturekit-rs |
| 截图遮罩 | Win32 transparent overlay | AppKit transparent NSWindow |
| 贴图窗口 | Win32 layered window | AppKit NSWindow |
| 鼠标穿透 | Win32 extended style / hit-test | ignoresMouseEvents / event handling |
| 置顶 | WS_EX_TOPMOST | NSWindow.Level / CoreGraphics window levels |
| 图像处理 | Rust image-core / WGPU optional | Rust image-core / Metal or WGPU optional |
| OCR 本地 | Tesseract / Windows OCR | Apple Vision / Tesseract |
| AI | Rust AI Provider | Rust AI Provider |
| 存储 | SQLite + encrypted files | SQLite + encrypted files |
| 打包 | MSI / MSIX | DMG / PKG + notarization |
| 权限重点 | UAC、剪贴板、全局热键、显示器 | Screen Recording、Accessibility、Automation |

---

## 7. AI 技术栈

AI 模块应独立于平台，实现为跨平台 Provider 架构。

```text
ai-core
├── VisionProvider trait
├── AiRequest
├── AiResponse
├── PromptTemplate
├── ConversationContext
├── ModelRouter
├── ImageCompressor
├── PrivacyGate
└── UsageLimiter
```

### 7.1 AI Provider

```text
ai-providers
├── OpenAICompatibleProvider
├── AzureOpenAIProvider
├── LocalVisionProvider
├── EnterpriseGatewayProvider
└── MockProvider
```

### 7.2 AI 功能入口

- 截图后 AI 分析。
- OCR 后 AI 总结。
- 贴图右键 AI 分析。
- 历史截图 AI 分析。
- 局部区域 AI 分析。
- 对同一截图多轮追问。

### 7.3 AI 隐私流程

```text
Image
→ Privacy Check
→ Optional Redaction
→ User Consent / Enterprise Policy
→ Compression
→ AI Provider
→ Result
```

---

## 8. OCR 技术栈

OCR 同样采用跨平台 Provider 架构。

```text
ocr-core
├── OcrProvider trait
├── OcrRequest
├── OcrResult
├── OcrBlock
├── OcrTable
├── OcrPostProcessor
└── OcrLanguage
```

### 8.1 OCR Provider

```text
ocr-providers
├── AppleVisionProvider
├── WindowsOcrProvider
├── TesseractProvider
├── AzureOcrProvider
├── TencentOcrProvider
├── OcrSpaceProvider
└── CustomHttpProvider
```

### 8.2 OCR 流程

```text
Image
→ Preprocess
→ Text Detection
→ OCR Provider
→ Postprocess
→ Structure Recovery
→ Result Panel
```

### 8.3 OCR 结果结构示例

```json
{
  "text": "识别出的完整文本",
  "language": "zh-CN",
  "blocks": [
    {
      "text": "文本块",
      "bbox": [0, 0, 200, 40],
      "confidence": 0.96
    }
  ],
  "tables": [],
  "created_at": "timestamp"
}
```

---

## 9. 安全与隐私技术栈

专业版需要从第一天设计隐私和安全，不建议后补。

```text
privacy-core
├── PrivacyMode
├── SensitiveDataDetector
├── RedactionEngine
├── UploadGuard
├── PolicyEngine
├── AuditLogger
└── SecureStorage
```

### 9.1 默认隐私策略

- 默认不上传图片。
- 默认不开启云端 AI。
- OCR 优先本地处理。
- 云端 OCR / AI 必须用户显式配置或授权。
- API Key 使用系统安全存储。
- 隐私模式下不保存截图、OCR、AI 历史。
- 企业策略可禁止云端上传。
- 分享前可强制脱敏扫描。

### 9.2 敏感信息检测

- 手机号。
- 邮箱。
- 身份证号。
- 银行卡号。
- Token。
- API Key。
- IP 地址。
- 主机名。
- 用户名。
- 二维码。

---

## 10. 存储技术栈

推荐 SQLite + 文件缓存 + 敏感配置加密。

```text
storage-core
├── settings.db
├── history.db
├── thumbnails/
├── captures/
├── ocr/
├── ai/
└── workspace/
```

### 10.1 存储内容

- 截图历史。
- 贴图状态。
- OCR 结果。
- AI 对话。
- Prompt 模板。
- 用户设置。
- 快捷键配置。
- 工作区状态。
- 许可证与功能开关。

### 10.2 隐私模式下的存储行为

- 不保存截图。
- 不保存 OCR 结果。
- 不保存 AI 结果。
- 不写历史数据库。
- 临时文件退出删除。

---

## 11. 打包、发布与更新

### 11.1 Windows

```text
Packaging:
- MSI
- MSIX optional

Signing:
- Windows Code Signing Certificate

Update:
- Tauri updater 或自研更新器

Enterprise:
- 静默安装
- Registry Policy
- Group Policy
- 离线安装包
```

### 11.2 macOS

```text
Packaging:
- DMG
- PKG optional

Signing:
- Apple Developer ID

Notarization:
- Apple Notarization

Update:
- Sparkle 或 Tauri updater

Permission:
- Screen Recording 权限引导
- Accessibility 权限引导
- Info.plist 权限描述
```

---

## 12. 推荐开发优先级

### Phase 0：技术验证

目标：验证最难的系统能力。

必须完成：

- Windows 区域截图 PoC。
- Windows 多屏 DPI PoC。
- Windows 透明遮罩窗口 PoC。
- Windows 置顶贴图窗口 PoC。
- Windows 鼠标穿透 PoC。
- macOS ScreenCaptureKit PoC。
- macOS AppKit 透明浮窗 PoC。
- macOS Screen Recording 权限引导 PoC。
- 全局快捷键 PoC。
- 图像保存/复制 PoC。

建议周期：2 到 4 周。

### Phase 1：Windows 专业核心

目标：先把 Windows 端做到可日用。

包含：

- 区域截图。
- 窗口识别。
- 多屏支持。
- 基础标注。
- 复制/保存。
- 贴图置顶。
- 贴图缩放。
- 贴图透明度。
- 鼠标穿透。
- 快捷键配置。

建议周期：6 到 8 周。

### Phase 2：macOS 专业核心

目标：完成 macOS 核心体验。

包含：

- ScreenCaptureKit 截图。
- Screen Recording 权限引导。
- Retina 坐标处理。
- AppKit 截图遮罩。
- AppKit 贴图窗口。
- 鼠标穿透。
- 基础标注。
- 复制/保存。

建议周期：6 到 8 周。

### Phase 3：OCR 与历史

目标：形成效率工具闭环。

包含：

- 本地 OCR。
- 云端 OCR Provider。
- OCR 结果面板。
- 截图历史。
- 贴图恢复。
- 基础搜索。
- 取色器。

建议周期：4 到 6 周。

### Phase 4：AI 识图

目标：形成差异化。

包含：

- AI 图片总结。
- AI 翻译。
- AI 报错分析。
- OCR + AI 联动。
- Prompt 模板。
- 多轮追问。
- AI 结果导出。

建议周期：4 到 6 周。

### Phase 5：企业与安全能力

目标：商业化和团队部署。

包含：

- 自动脱敏。
- 历史加密。
- 企业策略。
- 私有 AI 网关。
- MSI / PKG 静默安装。
- 许可证管理。
- 审计日志。

建议周期：持续迭代。

---

## 13. 最终技术栈建议

### 13.1 总体定案

```text
Rust Core + Tauri 2 管理界面
Windows: windows-rs + Windows Graphics Capture + DXGI fallback + Win32 Native Window
macOS: ScreenCaptureKit + AppKit + CoreGraphics + Vision + objc2 / Swift Bridge
```

### 13.2 一句话总结

> Rust 负责核心引擎，Windows/macOS 原生 API 负责系统级截图与贴图，Tauri/React 负责复杂业务界面。

### 13.3 这样设计的好处

- 截图和贴图体验可以做到专业级。
- Windows 与 macOS 可以分别深度优化。
- OCR、AI、隐私、安全、存储可以跨平台复用。
- 后续可以扩展企业版、本地 AI、插件系统。
- 不会被 WebView 的窗口能力限制。
- 有利于长期维护、测试、性能优化和商业化部署。

---

## 14. 参考资料

- Tauri 官方文档：Tauri 支持使用任意前端框架构建 UI，并通过 Rust 承担后端逻辑，同时使用系统 WebView 以减少包体。
- Microsoft windows-rs：用于从 Rust 直接调用 Windows API。
- windows-capture：Rust 屏幕捕获库，基于 Windows Graphics Capture API，并支持 DXGI Desktop Duplication API。
- Apple ScreenCaptureKit：macOS 高性能屏幕、窗口、应用捕获框架。
- screencapturekit-rs：Apple ScreenCaptureKit 的 Rust 绑定。
- Apple AppKit / CoreGraphics：macOS 原生窗口、事件、窗口层级能力。
- Apple Vision：macOS/iOS 端本地 OCR 与视觉识别框架。
- Rust objc2：Rust 与 Objective-C/AppKit/Foundation 等 Apple 平台框架互操作。
- Tesseract Rust bindings：用于跨平台本地 OCR 的可选方案。
