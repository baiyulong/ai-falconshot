use image::RgbaImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl PixelColor {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn hex_with_alpha(&self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }

    pub fn rgb_string(&self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    pub fn rgba_string(&self) -> String {
        format!(
            "rgba({}, {}, {}, {:.2})",
            self.r,
            self.g,
            self.b,
            self.a as f32 / 255.0
        )
    }

    pub fn hsl(&self) -> (f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;

        if (max - min).abs() < f32::EPSILON {
            return (0.0, 0.0, l * 100.0);
        }

        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };

        let h = if (max - r).abs() < f32::EPSILON {
            let mut h = (g - b) / d;
            if g < b {
                h += 6.0;
            }
            h
        } else if (max - g).abs() < f32::EPSILON {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };

        (h * 60.0, s * 100.0, l * 100.0)
    }

    pub fn hsl_string(&self) -> String {
        let (h, s, l) = self.hsl();
        format!("hsl({:.0}, {:.0}%, {:.0}%)", h, s, l)
    }
}

pub struct ColorPicker {
    history: Vec<PixelColor>,
    max_history: usize,
}

impl ColorPicker {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history: 20,
        }
    }

    pub fn pick_from_image(&mut self, image: &RgbaImage, x: u32, y: u32) -> Option<PixelColor> {
        if x >= image.width() || y >= image.height() {
            return None;
        }
        let px = image.get_pixel(x, y);
        let color = PixelColor::new(px[0], px[1], px[2], px[3]);
        self.add_to_history(color);
        Some(color)
    }

    pub fn average_color(&self, image: &RgbaImage, x: u32, y: u32, radius: u32) -> Option<PixelColor> {
        let mut r_sum: u64 = 0;
        let mut g_sum: u64 = 0;
        let mut b_sum: u64 = 0;
        let mut a_sum: u64 = 0;
        let mut count: u64 = 0;

        let x_start = x.saturating_sub(radius);
        let y_start = y.saturating_sub(radius);
        let x_end = (x + radius).min(image.width() - 1);
        let y_end = (y + radius).min(image.height() - 1);

        for py in y_start..=y_end {
            for px in x_start..=x_end {
                let pixel = image.get_pixel(px, py);
                r_sum += pixel[0] as u64;
                g_sum += pixel[1] as u64;
                b_sum += pixel[2] as u64;
                a_sum += pixel[3] as u64;
                count += 1;
            }
        }

        if count == 0 {
            return None;
        }

        Some(PixelColor::new(
            (r_sum / count) as u8,
            (g_sum / count) as u8,
            (b_sum / count) as u8,
            (a_sum / count) as u8,
        ))
    }

    pub fn add_to_history(&mut self, color: PixelColor) {
        self.history.retain(|c| *c != color);
        self.history.insert(0, color);
        if self.history.len() > self.max_history {
            self.history.truncate(self.max_history);
        }
    }

    pub fn history(&self) -> &[PixelColor] {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

impl Default for ColorPicker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_format() {
        let c = PixelColor::new(255, 128, 0, 255);
        assert_eq!(c.hex(), "#FF8000");
        assert_eq!(c.hex_with_alpha(), "#FF8000FF");
    }

    #[test]
    fn test_rgb_string() {
        let c = PixelColor::new(10, 20, 30, 255);
        assert_eq!(c.rgb_string(), "rgb(10, 20, 30)");
    }

    #[test]
    fn test_hsl_red() {
        let c = PixelColor::new(255, 0, 0, 255);
        let (h, s, l) = c.hsl();
        assert!((h - 0.0).abs() < 1.0);
        assert!((s - 100.0).abs() < 1.0);
        assert!((l - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_pick_from_image() {
        let img = RgbaImage::from_pixel(10, 10, image::Rgba([100, 150, 200, 255]));
        let mut picker = ColorPicker::new();
        let color = picker.pick_from_image(&img, 5, 5).unwrap();
        assert_eq!(color, PixelColor::new(100, 150, 200, 255));
        assert_eq!(picker.history().len(), 1);
    }

    #[test]
    fn test_pick_out_of_bounds() {
        let img = RgbaImage::new(10, 10);
        let mut picker = ColorPicker::new();
        assert!(picker.pick_from_image(&img, 20, 20).is_none());
    }

    #[test]
    fn test_average_color() {
        let img = RgbaImage::from_pixel(10, 10, image::Rgba([100, 100, 100, 255]));
        let picker = ColorPicker::new();
        let avg = picker.average_color(&img, 5, 5, 2).unwrap();
        assert_eq!(avg, PixelColor::new(100, 100, 100, 255));
    }

    #[test]
    fn test_history_dedup() {
        let mut picker = ColorPicker::new();
        let c = PixelColor::new(255, 0, 0, 255);
        picker.add_to_history(c);
        picker.add_to_history(PixelColor::new(0, 255, 0, 255));
        picker.add_to_history(c);
        assert_eq!(picker.history().len(), 2);
        assert_eq!(picker.history()[0], c);
    }
}
