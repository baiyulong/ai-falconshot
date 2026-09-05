<p align="center"><img src="docs/assets/logo-mark.png" alt="FalconShot logo" width="130" /></p>

# FalconShot

<p align="center">
  <a href="README.md">简体中文</a> | <strong>English</strong> | <a href="https://baiyulong.github.io/ai-falconshot/">Homepage</a> | <a href="https://baiyulong.github.io/ai-falconshot/privacy.html">Privacy</a>
</p>

A Windows desktop tool for smart screenshots, pinned images, OCR and AI vision.

Drag a region and the screenshot is captured the moment you release — annotate it, then **pin** it always-on-top, **extract text** (local OCR), or hand it to an **AI vision model** for structured Markdown output.

## Features

- **Region capture**: triggered by the global `F2` hotkey (customizable combo, pausable in Settings), drag a region then fine-tune its edges/corners or move it as a whole, confirm with `Enter` / double-click / the ✓ button, `Esc`/right-click to cancel, live size indicator
- **Freeze-frame capture**: the screen is frozen when capture mode starts — what you saw is what you get, unaffected by popups
- **Annotation editor**: rectangle, arrow, pen, highlighter and text with undo/redo; a borderless window overlays the shot exactly in place
- **Pin to screen**: turn any shot (with annotations) into an always-on-top floating image — drag freely, pin many at once, double-click to close
- **Text extraction (local OCR)**: built on Windows.Media.Ocr, offline and instant, mixed Chinese/English
- **AI recognition**: send shots to a vision-capable model and get Markdown output (tables become Markdown tables, code in code blocks); System Prompt is customizable
- **Clipboard**: copies are written as both DIB and PNG, so pasting into chat apps and documents just works
- **History**: screenshots are kept automatically for later reference
- **UI language**: 简体中文 / English, following the system language by default, switchable in Settings

## AI model configuration

Open **Settings → AI recognition model**:

| Setting | Description |
|---|---|
| API Key | Platform secret (or use the `FALCONSHOT_AI_KEY` environment variable) |
| Base URL | API root, up to `/v1`, e.g. `https://api.agnes-ai.cn/v1` |
| Model | **`agnes-2.5-flash` recommended** (free on the Agnes AI platform); must accept image input |
| System Prompt | Leave empty for the default OCR prompt, or customize the recognition style |

> The API is OpenAI-compatible `chat/completions` (images uploaded as base64), so the model must support vision input; text-only models will return an error. Once saved, click "AI Extract" in the editor.

## Shortcuts

| Shortcut | Action |
|---|---|
| `F2` (default, changeable) | Start region capture |
| `Esc` / right-click | Cancel capture / close editor |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo annotation |
| Double-click a pin | Close that pin |

## Tech stack

- **Rust workspace**: 14 layered crates (capture / overlay / floating / annotation / image / clipboard / hotkey / ocr / ai / privacy / storage / settings / platform-windows / app)
- **Tauri 2 + React 19 + TypeScript + Tailwind CSS**
- **Windows native**: GDI screen capture, Win32 layered windows (pins), global hotkeys, Windows.Media.Ocr

## Development

```bash
# Desktop app (incremental build, main window shows automatically)
cd apps/desktop-tauri && npm install && npm run tauri dev

# Rust checks and tests
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace

# Frontend type check
cd apps/desktop-tauri && npx tsc --noEmit
```

## Packaging

After pushing, GitHub Actions builds the MSI installer on a Windows runner:

- Push a `v*` tag (e.g. `v0.1.0`) → a GitHub Release is created automatically with the MSI and SHA256 checksums
- Trigger the Release workflow manually from the Actions page → download from the run's Artifacts

## Security note

Installers are **not code-signed yet** (a signing certificate for the individual developer is planned), therefore:

- Windows shows an **Unknown Publisher** prompt during installation: verify the download comes from this repository's Releases page, then click "More info" → "Run anyway" on the SmartScreen dialog
- Some antivirus products may **flag it** due to the missing reputation: restore/allow it in your antivirus; all source code in this repository is public and installers are built by GitHub Actions from source
- After downloading, verify the SHA256 against the `checksums.txt` attached to the Release (or the hashes in the release notes)

If you hit a false positive, please open an issue — I will file false-positive reports with Microsoft.

## License

MIT
