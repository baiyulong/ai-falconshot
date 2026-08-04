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

impl AnnotationObject {
    pub fn contains_point(&self, x: f32, y: f32, tolerance: f32) -> bool {
        match self {
            Self::Rectangle { rect, .. }
            | Self::RoundedRect { rect, .. }
            | Self::Ellipse { rect, .. }
            | Self::Mosaic { rect, .. }
            | Self::GaussianBlur { rect, .. } => {
                let expanded = Rect::new(
                    rect.x - tolerance as i32,
                    rect.y - tolerance as i32,
                    rect.width + (tolerance * 2.0) as u32,
                    rect.height + (tolerance * 2.0) as u32,
                );
                expanded.contains_point(x as i32, y as i32)
            }
            Self::Line { start, end, .. } | Self::Arrow { start, end, .. } => {
                distance_to_segment(x, y, start.0, start.1, end.0, end.1) <= tolerance
            }
            Self::Polyline { points, .. }
            | Self::Freehand { points, .. }
            | Self::Highlighter { points, .. } => {
                for window in points.windows(2) {
                    let (x1, y1) = window[0];
                    let (x2, y2) = window[1];
                    if distance_to_segment(x, y, x1, y1, x2, y2) <= tolerance {
                        return true;
                    }
                }
                false
            }
            Self::Text {
                position,
                content,
                style,
            } => {
                let w = content.len() as f32 * style.font_size * 0.6;
                let h = style.font_size * 1.4;
                x >= position.0 - tolerance
                    && x <= position.0 + w + tolerance
                    && y >= position.1 - tolerance
                    && y <= position.1 + h + tolerance
            }
            Self::NumberMarker { position, .. } => {
                let r = 12.0 + tolerance;
                let dx = x - position.0;
                let dy = y - position.1;
                dx * dx + dy * dy <= r * r
            }
        }
    }

    pub fn bounding_rect(&self) -> Rect {
        match self {
            Self::Rectangle { rect, .. }
            | Self::RoundedRect { rect, .. }
            | Self::Ellipse { rect, .. }
            | Self::Mosaic { rect, .. }
            | Self::GaussianBlur { rect, .. } => rect.clone(),
            Self::Line { start, end, .. } | Self::Arrow { start, end, .. } => {
                let x = start.0.min(end.0) as i32;
                let y = start.1.min(end.1) as i32;
                let w = (start.0 - end.0).abs() as u32;
                let h = (start.1 - end.1).abs() as u32;
                Rect::new(x, y, w.max(1), h.max(1))
            }
            Self::Polyline { points, .. }
            | Self::Freehand { points, .. }
            | Self::Highlighter { points, .. } => points_bounds(points),
            Self::Text {
                position,
                content,
                style,
            } => {
                let w = (content.len() as f32 * style.font_size * 0.6) as u32;
                let h = (style.font_size * 1.4) as u32;
                Rect::new(position.0 as i32, position.1 as i32, w.max(1), h.max(1))
            }
            Self::NumberMarker { position, .. } => {
                Rect::new(position.0 as i32 - 12, position.1 as i32 - 12, 24, 24)
            }
        }
    }
}

fn points_bounds(points: &[(f32, f32)]) -> Rect {
    if points.is_empty() {
        return Rect::new(0, 0, 1, 1);
    }
    let min_x = points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let min_y = points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let max_x = points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let max_y = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
    Rect::new(
        min_x as i32,
        min_y as i32,
        ((max_x - min_x) as u32).max(1),
        ((max_y - min_y) as u32).max(1),
    )
}

fn distance_to_segment(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        return ((px - x1) * (px - x1) + (py - y1) * (py - y1)).sqrt();
    }
    let t = ((px - x1) * dx + (py - y1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;
    ((px - proj_x) * (px - proj_x) + (py - proj_y) * (py - proj_y)).sqrt()
}
