use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationStyle {
    pub stroke_color: [u8; 4],
    pub fill_color: Option<[u8; 4]>,
    pub stroke_width: f32,
    pub opacity: f32,
    pub font_family: String,
    pub font_size: f32,
    pub arrow_head: ArrowHeadStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArrowHeadStyle {
    Solid,
    Open,
    None,
}

impl Default for AnnotationStyle {
    fn default() -> Self {
        Self {
            stroke_color: [255, 0, 0, 255],
            fill_color: None,
            stroke_width: 3.0,
            opacity: 1.0,
            font_family: "Microsoft YaHei".to_string(),
            font_size: 16.0,
            arrow_head: ArrowHeadStyle::Solid,
        }
    }
}
