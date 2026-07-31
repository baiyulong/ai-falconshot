use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayMode {
    RegionSelect,
    WindowDetect,
    ElementDetect,
    ColorPick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub mask_color: [u8; 4],
    pub border_color: [u8; 4],
    pub border_width: u32,
    pub show_magnifier: bool,
    pub show_dimensions: bool,
    pub magnifier_size: u32,
    pub magnifier_zoom: f32,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            mask_color: [0, 0, 0, 100],
            border_color: [0, 174, 255, 255],
            border_width: 2,
            show_magnifier: true,
            show_dimensions: true,
            magnifier_size: 150,
            magnifier_zoom: 4.0,
        }
    }
}
