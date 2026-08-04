use capture_core::Rect;
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

#[derive(Debug, Clone)]
pub struct MagnifierState {
    pub x: i32,
    pub y: i32,
    pub pixel_color: [u8; 4],
    pub visible: bool,
}

impl MagnifierState {
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            pixel_color: [0, 0, 0, 255],
            visible: false,
        }
    }

    pub fn hex_color(&self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            self.pixel_color[0], self.pixel_color[1], self.pixel_color[2]
        )
    }
}

impl Default for MagnifierState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedWindow {
    pub rect: Rect,
    pub title: String,
    pub is_hovered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayAction {
    None,
    Selecting,
    Resizing,
    Moving,
    WindowHover,
    ColorPicking,
    Confirmed,
    Cancelled,
}
