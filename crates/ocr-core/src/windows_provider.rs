use crate::provider::OcrProvider;
use crate::types::{OcrBlock, OcrBlockType, OcrRequest, OcrResult};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Command;

pub struct WindowsOcrProvider {
    language: String,
}

impl WindowsOcrProvider {
    pub fn new() -> Self {
        Self {
            language: "zh-Hans".to_string(),
        }
    }

    pub fn with_language(language: &str) -> Self {
        Self {
            language: language.to_string(),
        }
    }

    fn run_powershell_ocr(&self, image_path: &str) -> Result<String> {
        let script = format!(
            r#"
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Media.Ocr.OcrEngine,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]
$null = [Windows.Storage.StorageFile,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]

AsTask([Windows.Storage.StorageFile]::GetFileFromPathAsync('{path}')) | Out-Null
$file = [Windows.Storage.StorageFile]::GetFileFromPathAsync('{path}').GetAwaiter().GetResult()
$stream = $file.OpenAsync([Windows.Storage.FileAccessMode]::Read).GetAwaiter().GetResult()
$decoder = [Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream).GetAwaiter().GetResult()
$bitmap = $decoder.GetSoftwareBitmapAsync().GetAwaiter().GetResult()
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
$result = $engine.RecognizeAsync($bitmap).GetAwaiter().GetResult()
Write-Output $result.Text
"#,
            path = image_path.replace('\'', "''")
        );

        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("PowerShell OCR failed: {stderr}")
        }
    }
}

impl Default for WindowsOcrProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OcrProvider for WindowsOcrProvider {
    fn name(&self) -> &str {
        "windows-ocr"
    }

    fn is_local(&self) -> bool {
        true
    }

    async fn recognize(&self, request: &OcrRequest) -> Result<OcrResult> {
        let start = std::time::Instant::now();

        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("falconshot_ocr_{}.png", std::process::id()));
        std::fs::write(&temp_path, &request.image_data)?;

        let result = tokio::task::spawn_blocking({
            let path = temp_path.to_string_lossy().to_string();
            let lang = self.language.clone();
            move || {
                let provider = WindowsOcrProvider::with_language(&lang);
                provider.run_powershell_ocr(&path)
            }
        })
        .await??;

        let _ = std::fs::remove_file(&temp_path);
        let duration = start.elapsed().as_millis() as u64;

        let blocks: Vec<OcrBlock> = result
            .lines()
            .filter(|l| !l.trim().is_empty())
            .enumerate()
            .map(|(i, line)| OcrBlock {
                text: line.to_string(),
                bbox: [0.0, i as f32 * 20.0, 100.0, 20.0],
                confidence: 0.9,
                block_type: OcrBlockType::Text,
            })
            .collect();

        Ok(OcrResult {
            text: result,
            language: self.language.clone(),
            blocks,
            tables: vec![],
            confidence: 0.9,
            duration_ms: duration,
        })
    }

    fn supported_languages(&self) -> Vec<String> {
        vec![
            "zh-Hans".to_string(),
            "zh-Hant".to_string(),
            "en".to_string(),
            "ja".to_string(),
            "ko".to_string(),
        ]
    }
}
