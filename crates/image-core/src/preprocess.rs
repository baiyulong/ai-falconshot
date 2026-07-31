use image::{DynamicImage, GrayImage, RgbaImage};

pub struct OcrPreprocessor;

impl OcrPreprocessor {
    pub fn to_grayscale(image: &RgbaImage) -> GrayImage {
        DynamicImage::ImageRgba8(image.clone()).to_luma8()
    }

    pub fn binarize(image: &GrayImage, threshold: u8) -> GrayImage {
        let mut output = image.clone();
        for pixel in output.pixels_mut() {
            let v = pixel[0];
            pixel[0] = if v > threshold { 255 } else { 0 };
        }
        output
    }

    pub fn enhance_contrast(image: &GrayImage) -> GrayImage {
        let mut output = image.clone();
        let (min, max) = image
            .pixels()
            .fold((255u8, 0u8), |(mn, mx), p| (mn.min(p[0]), mx.max(p[0])));

        if max == min {
            return output;
        }

        let range = (max - min) as f32;
        for pixel in output.pixels_mut() {
            let normalized = ((pixel[0] - min) as f32 / range * 255.0) as u8;
            pixel[0] = normalized;
        }
        output
    }
}
