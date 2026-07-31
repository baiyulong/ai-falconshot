use image::{DynamicImage, GenericImageView, RgbaImage};

pub fn crop(image: &RgbaImage, x: u32, y: u32, width: u32, height: u32) -> RgbaImage {
    image.view(x, y, width, height).to_image()
}

pub fn resize(image: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let dyn_img = DynamicImage::ImageRgba8(image.clone());
    dyn_img
        .resize_exact(width, height, image::imageops::FilterType::Lanczos3)
        .to_rgba8()
}

pub fn rotate_90(image: &RgbaImage) -> RgbaImage {
    let dyn_img = DynamicImage::ImageRgba8(image.clone());
    dyn_img.rotate90().to_rgba8()
}

pub fn flip_horizontal(image: &RgbaImage) -> RgbaImage {
    let dyn_img = DynamicImage::ImageRgba8(image.clone());
    dyn_img.fliph().to_rgba8()
}

pub fn flip_vertical(image: &RgbaImage) -> RgbaImage {
    let dyn_img = DynamicImage::ImageRgba8(image.clone());
    dyn_img.flipv().to_rgba8()
}
