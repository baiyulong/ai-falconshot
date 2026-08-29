use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    pub fn center(&self) -> (i32, i32) {
        (
            self.x + self.width as i32 / 2,
            self.y + self.height as i32 / 2,
        )
    }

    pub fn contains_point(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if right > x && bottom > y {
            Some(Rect {
                x,
                y,
                width: (right - x) as u32,
                height: (bottom - y) as u32,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureOptions {
    pub region: Option<Rect>,
    pub monitor_index: Option<usize>,
    pub include_cursor: bool,
    pub exclude_self: bool,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            region: None,
            monitor_index: None,
            include_cursor: false,
            exclude_self: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CaptureFrame {
    pub image: image::RgbaImage,
    pub monitor: MonitorInfo,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub bounds: Rect,
    pub work_area: Rect,
    pub scale_factor: f64,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u64,
    pub title: String,
    pub class_name: String,
    pub bounds: Rect,
    pub is_visible: bool,
    pub is_minimized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_new() {
        let r = Rect::new(10, 20, 100, 200);
        assert_eq!(r.x, 10);
        assert_eq!(r.y, 20);
        assert_eq!(r.width, 100);
        assert_eq!(r.height, 200);
    }

    #[test]
    fn rect_is_empty() {
        assert!(Rect::new(0, 0, 0, 100).is_empty());
        assert!(Rect::new(0, 0, 100, 0).is_empty());
        assert!(!Rect::new(0, 0, 1, 1).is_empty());
    }

    #[test]
    fn rect_right_bottom() {
        let r = Rect::new(-100, -50, 300, 200);
        assert_eq!(r.right(), 200);
        assert_eq!(r.bottom(), 150);
    }

    #[test]
    fn rect_center() {
        let r = Rect::new(0, 0, 100, 200);
        assert_eq!(r.center(), (50, 100));

        let r2 = Rect::new(-100, -100, 200, 200);
        assert_eq!(r2.center(), (0, 0));
    }

    #[test]
    fn rect_contains_point() {
        let r = Rect::new(10, 10, 100, 100);
        assert!(r.contains_point(10, 10));
        assert!(r.contains_point(50, 50));
        assert!(r.contains_point(109, 109));
        assert!(!r.contains_point(110, 110));
        assert!(!r.contains_point(9, 10));
        assert!(!r.contains_point(10, 9));
    }

    #[test]
    fn rect_contains_point_negative_coords() {
        let r = Rect::new(-1920, -1080, 1920, 1080);
        assert!(r.contains_point(-1920, -1080));
        assert!(r.contains_point(-1, -1));
        assert!(!r.contains_point(0, 0));
        assert!(!r.contains_point(-1921, -1080));
    }

    #[test]
    fn rect_intersects() {
        let a = Rect::new(0, 0, 100, 100);
        let b = Rect::new(50, 50, 100, 100);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));

        let c = Rect::new(200, 200, 50, 50);
        assert!(!a.intersects(&c));

        let d = Rect::new(100, 0, 50, 50);
        assert!(!a.intersects(&d)); // touching edge is not intersection
    }

    #[test]
    fn rect_intersection() {
        let a = Rect::new(0, 0, 100, 100);
        let b = Rect::new(50, 50, 100, 100);
        let inter = a.intersection(&b).unwrap();
        assert_eq!(inter, Rect::new(50, 50, 50, 50));

        let c = Rect::new(200, 200, 50, 50);
        assert!(a.intersection(&c).is_none());
    }

    #[test]
    fn rect_intersection_negative_coords() {
        let a = Rect::new(-100, -100, 200, 200);
        let b = Rect::new(0, 0, 200, 200);
        let inter = a.intersection(&b).unwrap();
        assert_eq!(inter, Rect::new(0, 0, 100, 100));
    }

    #[test]
    fn capture_options_default() {
        let opts = CaptureOptions::default();
        assert!(opts.region.is_none());
        assert!(opts.monitor_index.is_none());
        assert!(!opts.include_cursor);
        assert!(opts.exclude_self);
    }

    #[test]
    fn rect_serialization_roundtrip() {
        let r = Rect::new(-1920, 0, 3840, 2160);
        let json = serde_json::to_string(&r).unwrap();
        let deserialized: Rect = serde_json::from_str(&json).unwrap();
        assert_eq!(r, deserialized);
    }

    #[test]
    fn monitor_info_serialization() {
        let m = MonitorInfo {
            index: 0,
            name: "\\\\DISPLAY1".to_string(),
            bounds: Rect::new(0, 0, 1920, 1080),
            work_area: Rect::new(0, 0, 1920, 1040),
            scale_factor: 1.5,
            is_primary: true,
        };
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: MonitorInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "\\\\DISPLAY1");
        assert_eq!(deserialized.scale_factor, 1.5);
        assert!(deserialized.is_primary);
    }

    #[test]
    fn window_info_serialization() {
        let w = WindowInfo {
            id: 12345,
            title: "Test Window".to_string(),
            class_name: "TestClass".to_string(),
            bounds: Rect::new(100, 100, 800, 600),
            is_visible: true,
            is_minimized: false,
        };
        let json = serde_json::to_string(&w).unwrap();
        let deserialized: WindowInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 12345);
        assert_eq!(deserialized.title, "Test Window");
        assert!(!deserialized.is_minimized);
    }
}
