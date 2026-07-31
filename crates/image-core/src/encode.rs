use anyhow::Result;
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Png,
    Jpg,
    WebP,
    Bmp,
}

impl ImageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::WebP => "webp",
            Self::Bmp => "bmp",
        }
    }
}

pub fn save_image(image: &RgbaImage, path: &Path, format: ImageFormat) -> Result<()> {
    let encoder_format = match format {
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::Jpg => image::ImageFormat::Jpeg,
        ImageFormat::WebP => image::ImageFormat::WebP,
        ImageFormat::Bmp => image::ImageFormat::Bmp,
    };
    image.save_with_format(path, encoder_format)?;
    Ok(())
}
