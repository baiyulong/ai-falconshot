<p align="center"><img src="docs/assets/logo-mark.png" alt="FalconShot logo" width="130" /></p>

# FalconShot

<p align="center">
  <strong>简体中文</strong> | <a href="README.en.md">English</a> | <a href="https://baiyulong.github.io/ai-falconshot/">项目主页</a> | <a href="https://baiyulong.github.io/ai-falconshot/privacy.html">隐私政策</a>
</p>

智能截图、贴图、OCR 与 AI 视觉识别的 Windows 桌面工具。

拖出选区松手即得截图，标注后可一键**贴图**置顶、**提取文本**（本地 OCR）、或交给 **AI 视觉模型**输出 Markdown 结构化内容。

## 功能

- **区域截图**：F2 全局快捷键触发（可在设置中自定义组合键、支持暂停），拖出选区后可拖拽四边/四角调整大小、整体移动，回车 / 双击 / 点击 ✓ 确认，Esc/右键取消，实时显示尺寸
- **冻结帧捕获**：进入截图模式时冻结屏幕画面，所见即所得，不被后续弹窗干扰
- **标注编辑器**：矩形、箭头、画笔、荧光笔、文字，撤销/重做，无边框窗口精确覆盖在截图原位
- **贴图**：截图（含标注）一键变为置顶悬浮图，可任意拖动，支持多张贴图，双击关闭
- **提取文本（本地 OCR）**：基于 Windows.Media.Ocr，离线秒出，中英文混排
- **AI 识别**：将截图发给支持视觉的大模型，按 Markdown 输出（表格转 Markdown 表格、代码块），System Prompt 可自定义
- **剪贴板**：复制同时写入 DIB 与 PNG 格式，粘贴到聊天窗口、文档均可用
- **历史记录**：截图自动留存，可回溯
- **界面语言**：简体中文 / English，默认跟随系统语言，可在设置中切换

## AI 模型配置

打开 **设置 → AI 识别模型**：

| 配置项 | 说明 |
|---|---|
| API Key | 平台密钥（也可用环境变量 `FALCONSHOT_AI_KEY`） |
| Base URL | API 根地址，到 `/v1` 为止，如 `https://api.agnes-ai.cn/v1` |
| 模型 | **推荐 `agnes-2.5-flash`**（Agnes AI 平台目前免费），需支持图片输入 |
| System Prompt | 留空使用默认的 OCR 提示词，可自定义识别风格 |

> 接口为 OpenAI 兼容的 `chat/completions`（图片以 base64 上传），因此模型必须支持视觉输入；纯文本模型会返回报错。保存后即可在编辑器中点击"AI 识别"。

## 快捷键

| 快捷键 | 功能 |
|---|---|
| `F2`（默认，可改） | 开始区域截图 |
| `Esc` / 右键 | 取消截图 / 关闭编辑器 |
| `Ctrl+Z` / `Ctrl+Y` | 撤销 / 重做标注 |
| 双击贴图 | 关闭该贴图 |

## 技术栈

- **Rust Workspace**：14 个 crate 分层（capture / overlay / floating / annotation / image / clipboard / hotkey / ocr / ai / privacy / storage / settings / platform-windows / app）
- **Tauri 2 + React 19 + TypeScript + Tailwind CSS**
- **Windows 原生**：GDI 屏幕捕获、Win32 分层窗口（贴图）、全局热键、Windows.Media.Ocr

## 开发

```bash
# 桌面应用（增量编译，主窗口自动显示）
cd apps/desktop-tauri && npm install && npm run tauri dev

# Rust 检查与测试
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace

# 前端类型检查
cd apps/desktop-tauri && npx tsc --noEmit
```

## 打包

推送到远端后，GitHub Actions 在 Windows runner 上构建 MSI 安装包：

- 推送 `v*` 标签（如 `v0.1.0`）→ 自动创建 GitHub Release 并附上 MSI 与 SHA256 校验值
- Actions 页面手动触发 Release 工作流 → 从 Run 的 Artifacts 下载

## 安全性说明

安装包**尚未做代码签名**（个人开发者的签名证书在规划中），因此：

- 安装时 Windows 会提示 **Unknown Publisher**：确认下载地址是本仓库 Releases 页即可继续（SmartScreen 界面点"More info / 更多信息" → "Run anyway / 仍要运行"）
- 部分杀毒软件可能因缺乏信誉记录而**误报**：可在杀软中恢复/加白名单；本仓库全部源码公开，安装包由 GitHub Actions 从源码构建
- 下载后建议核对 SHA256：与 Release 附带的 `checksums.txt`（或 Release 说明中的哈希值）一致即为本仓库原版

若遇到误报，欢迎提 issue，我会向 Microsoft 提交误报申诉。

## 许可证

MIT
