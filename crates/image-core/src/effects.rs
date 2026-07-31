use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

pub fn apply_mosaic(
    image: &mut RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    block_size: u32,
) {
    let (img_w, img_h) = image.dimensions();
    let x_end = (x + width).min(img_w);
    let y_end = (y + height).min(img_h);

    let mut by = y;
    while by < y_end {
        let mut bx = x;
        while bx < x_end {
            let bw = block_size.min(x_end - bx);
            let bh = block_size.min(y_end - by);
            let avg = average_color(image, bx, by, bw, bh);
            fill_block(image, bx, by, bw, bh, avg);
            bx += block_size;
        }
        by += block_size;
    }
}

fn average_color(image: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> Rgba<u8> {
    let mut r: u64 = 0;
    let mut g: u64 = 0;
    let mut b: u64 = 0;
    let mut count: u64 = 0;

    for py in y..(y + h) {
        for px in x..(x + w) {
            let pixel = image.get_pixel(px, py);
            r += pixel[0] as u64;
            g += pixel[1] as u64;
            b += pixel[2] as u64;
            count += 1;
        }
    }

    if count == 0 {
        return Rgba([0, 0, 0, 255]);
    }
    Rgba([(r / count) as u8, (g / count) as u8, (b / count) as u8, 255])
}

fn fill_block(image: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    for py in y..(y + h) {
        for px in x..(x + w) {
            image.put_pixel(px, py, color);
        }
    }
}

pub fn apply_gaussian_blur(
    image: &mut RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    radius: f32,
) {
    let (img_w, img_h) = image.dimensions();
    let x_end = (x + width).min(img_w);
    let y_end = (y + height).min(img_h);
    let region = image.view(x, y, x_end - x, y_end - y).to_image();
    let dyn_region = DynamicImage::ImageRgba8(region);
    let blurred = dyn_region.blur(radius).to_rgba8();

    for py in 0..blurred.height() {
        for px in 0..blurred.width() {
            image.put_pixel(x + px, y + py, *blurred.get_pixel(px, py));
        }
    }
}
