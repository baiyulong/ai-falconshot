use crate::style::AnnotationStyle;
use capture_core::Rect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationObject {
    Rectangle {
        rect: Rect,
        style: AnnotationStyle,
    },
    RoundedRect {
        rect: Rect,
        radius: f32,
        style: AnnotationStyle,
    },
    Ellipse {
        rect: Rect,
        style: AnnotationStyle,
    },
    Line {
        start: (f32, f32),
        end: (f32, f32),
        style: AnnotationStyle,
    },
    Arrow {
        start: (f32, f32),
        end: (f32, f32),
        style: AnnotationStyle,
    },
    Polyline {
        points: Vec<(f32, f32)>,
        style: AnnotationStyle,
    },
    Freehand {
        points: Vec<(f32, f32)>,
        style: AnnotationStyle,
    },
    Highlighter {
        points: Vec<(f32, f32)>,
        style: AnnotationStyle,
    },
    Text {
        position: (f32, f32),
        content: String,
        style: AnnotationStyle,
    },
    NumberMarker {
        position: (f32, f32),
        number: u32,
        style: AnnotationStyle,
    },
    Mosaic {
        rect: Rect,
        block_size: u32,
    },
    GaussianBlur {
        rect: Rect,
        radius: f32,
    },
}
