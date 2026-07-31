use crate::types::{CaptureFrame, CaptureOptions, MonitorInfo, WindowInfo};
use anyhow::Result;

pub trait CaptureBackend: Send + Sync {
    fn capture_region(&self, options: &CaptureOptions) -> Result<CaptureFrame>;
    fn capture_fullscreen(&self) -> Result<CaptureFrame>;
    fn capture_window(&self, window_id: u64) -> Result<CaptureFrame>;
    fn enumerate_monitors(&self) -> Result<Vec<MonitorInfo>>;
    fn enumerate_windows(&self) -> Result<Vec<WindowInfo>>;
}
