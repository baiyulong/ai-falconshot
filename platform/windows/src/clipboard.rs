use anyhow::Result;
use clipboard_core::{ClipboardBackend, ClipboardContent};
use image::RgbaImage;

#[cfg(windows)]
mod win {
    use super::*;
    use std::ptr;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
    use windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW,
        SetClipboardData,
    };
    use windows::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
    };

    const CF_DIB: u32 = 8;
    const CF_UNICODETEXT: u32 = 13;
    const BI_RGB: u32 = 0;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn set_data(fmt: u32, bytes: &[u8]) -> Result<()> {
        let hmem = GlobalAlloc(GMEM_MOVEABLE, bytes.len())
            .map_err(|e| anyhow::anyhow!("GlobalAlloc failed: {e}"))?;
        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            let _ = GlobalFree(Some(hmem));
            anyhow::bail!("GlobalLock failed");
        }
        ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
        let _ = GlobalUnlock(hmem);
        SetClipboardData(fmt, Some(HANDLE(hmem.0)))
            .map_err(|e| anyhow::anyhow!("SetClipboardData failed: {e}"))?;
        Ok(())
    }

    pub struct WindowsClipboardBackend;

    impl Default for WindowsClipboardBackend {
        fn default() -> Self {
            Self::new()
        }
    }

    impl WindowsClipboardBackend {
        pub fn new() -> Self {
            Self
        }

        fn open(&self) -> Result<()> {
            unsafe {
                OpenClipboard(None).map_err(|e| anyhow::anyhow!("OpenClipboard failed: {e}"))?;
            }
            Ok(())
        }

        fn close(&self) {
            unsafe {
                let _ = CloseClipboard();
            }
        }
    }

    impl ClipboardBackend for WindowsClipboardBackend {
        fn set_image(&self, image: &RgbaImage) -> Result<()> {
            self.open()?;

            // Both a 24-bit DIB (universally accepted) and a PNG blob
            // (preferred by many modern apps) go onto the clipboard.
            let result = unsafe {
                let _ = EmptyClipboard();

                let width = image.width();
                let height = image.height();
                let row_size = (width * 3).div_ceil(4) * 4;
                let header_size = std::mem::size_of::<BITMAPINFOHEADER>();
                let total_size = header_size + (row_size * height) as usize;

                let dib: Result<()> = (|| {
                    let hmem = GlobalAlloc(GMEM_MOVEABLE, total_size)
                        .map_err(|e| anyhow::anyhow!("GlobalAlloc failed: {e}"))?;

                    let ptr = GlobalLock(hmem);
                    if ptr.is_null() {
                        let _ = GlobalFree(Some(hmem));
                        anyhow::bail!("GlobalLock failed");
                    }

                    let header = &mut *(ptr as *mut BITMAPINFOHEADER);
                    *header = BITMAPINFOHEADER {
                        biSize: header_size as u32,
                        biWidth: width as i32,
                        biHeight: -(height as i32),
                        biPlanes: 1,
                        biBitCount: 24,
                        biCompression: BI_RGB,
                        biSizeImage: row_size * height,
                        biXPelsPerMeter: 0,
                        biYPelsPerMeter: 0,
                        biClrUsed: 0,
                        biClrImportant: 0,
                    };

                    let pixel_ptr = (ptr as *mut u8).add(header_size);
                    for y in 0..height {
                        let row_offset = (y * row_size) as usize;
                        for x in 0..width {
                            let px = image.get_pixel(x, y);
                            let col_offset = row_offset + (x * 3) as usize;
                            *pixel_ptr.add(col_offset) = px[2]; // B
                            *pixel_ptr.add(col_offset + 1) = px[1]; // G
                            *pixel_ptr.add(col_offset + 2) = px[0]; // R
                        }
                    }

                    let _ = GlobalUnlock(hmem);
                    SetClipboardData(CF_DIB, Some(HANDLE(hmem.0)))
                        .map_err(|e| anyhow::anyhow!("SetClipboardData failed: {e}"))?;
                    Ok(())
                })();

                if let Err(e) = dib {
                    self.close();
                    return Err(e);
                }

                let mut png_buf = std::io::Cursor::new(Vec::new());
                let png: Result<()> = image::DynamicImage::ImageRgba8(image.clone())
                    .write_to(&mut png_buf, image::ImageFormat::Png)
                    .map_err(|e| anyhow::anyhow!("PNG encode failed: {e}"))
                    .and_then(|()| {
                        let fmt_name = wide("PNG");
                        let fmt = RegisterClipboardFormatW(PCWSTR(fmt_name.as_ptr()));
                        set_data(fmt, png_buf.get_ref())
                    });
                if let Err(e) = png {
                    self.close();
                    return Err(e);
                }

                Ok(())
            };

            self.close();
            result
        }

        fn get_image(&self) -> Result<Option<RgbaImage>> {
            self.open()?;

            let result = unsafe {
                let handle = match GetClipboardData(CF_DIB) {
                    Ok(h) => h,
                    Err(_) => {
                        self.close();
                        return Ok(None);
                    }
                };

                let hglobal = HGLOBAL(handle.0);
                let ptr = GlobalLock(hglobal);
                if ptr.is_null() {
                    self.close();
                    return Ok(None);
                }

                let header = &*(ptr as *const BITMAPINFOHEADER);
                let width = header.biWidth as u32;
                let height = header.biHeight.unsigned_abs();
                let bpp = header.biBitCount;
                let top_down = header.biHeight < 0;

                let header_size = header.biSize as usize;
                let pixel_ptr = (ptr as *const u8).add(header_size);

                let img = if bpp == 32 {
                    let mut img = RgbaImage::new(width, height);
                    let row_stride = width as usize * 4;
                    for y in 0..height {
                        let src_y = if top_down { y } else { height - 1 - y };
                        let row_offset = src_y as usize * row_stride;
                        for x in 0..width {
                            let offset = row_offset + x as usize * 4;
                            let b = *pixel_ptr.add(offset);
                            let g = *pixel_ptr.add(offset + 1);
                            let r = *pixel_ptr.add(offset + 2);
                            let a = *pixel_ptr.add(offset + 3);
                            img.put_pixel(x, y, image::Rgba([r, g, b, a]));
                        }
                    }
                    img
                } else if bpp == 24 {
                    let row_stride = (width as usize * 3).div_ceil(4) * 4;
                    let mut img = RgbaImage::new(width, height);
                    for y in 0..height {
                        let src_y = if top_down { y } else { height - 1 - y };
                        let row_offset = src_y as usize * row_stride;
                        for x in 0..width {
                            let offset = row_offset + x as usize * 3;
                            let b = *pixel_ptr.add(offset);
                            let g = *pixel_ptr.add(offset + 1);
                            let r = *pixel_ptr.add(offset + 2);
                            img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                        }
                    }
                    img
                } else {
                    let _ = GlobalUnlock(hglobal);
                    self.close();
                    return Ok(None);
                };

                let _ = GlobalUnlock(hglobal);
                Some(img)
            };

            self.close();
            Ok(result)
        }

        fn set_text(&self, text: &str) -> Result<()> {
            self.open()?;

            unsafe {
                let _ = EmptyClipboard();

                let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
                let byte_len = wide.len() * 2;

                let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len)
                    .map_err(|e| anyhow::anyhow!("GlobalAlloc failed: {e}"))?;

                let ptr = GlobalLock(hmem) as *mut u16;
                if ptr.is_null() {
                    let _ = GlobalFree(Some(hmem));
                    self.close();
                    anyhow::bail!("GlobalLock failed");
                }

                ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
                let _ = GlobalUnlock(hmem);

                SetClipboardData(CF_UNICODETEXT, Some(HANDLE(hmem.0)))
                    .map_err(|e| anyhow::anyhow!("SetClipboardData failed: {e}"))?;
            }

            self.close();
            Ok(())
        }

        fn get_text(&self) -> Result<Option<String>> {
            self.open()?;

            let result = unsafe {
                let handle = match GetClipboardData(CF_UNICODETEXT) {
                    Ok(h) => h,
                    Err(_) => {
                        self.close();
                        return Ok(None);
                    }
                };

                let hglobal = HGLOBAL(handle.0);
                let ptr = GlobalLock(hglobal) as *const u16;
                if ptr.is_null() {
                    self.close();
                    return Ok(None);
                }

                let size = GlobalSize(hglobal);
                let max_chars = size / 2;
                let mut len = 0;
                while len < max_chars && *ptr.add(len) != 0 {
                    len += 1;
                }

                let slice = std::slice::from_raw_parts(ptr, len);
                let text = String::from_utf16_lossy(slice);

                let _ = GlobalUnlock(hglobal);
                Some(text)
            };

            self.close();
            Ok(result)
        }

        fn get_content_type(&self) -> Result<ClipboardContent> {
            self.open()?;

            let content = unsafe {
                if GetClipboardData(CF_DIB).is_ok() {
                    ClipboardContent::Image
                } else if GetClipboardData(CF_UNICODETEXT).is_ok() {
                    ClipboardContent::Text
                } else {
                    ClipboardContent::Empty
                }
            };

            self.close();
            Ok(content)
        }

        fn clear(&self) -> Result<()> {
            self.open()?;
            unsafe {
                let _ = EmptyClipboard();
            }
            self.close();
            Ok(())
        }
    }
}

#[cfg(windows)]
pub use win::WindowsClipboardBackend;

#[cfg(not(windows))]
pub struct WindowsClipboardBackend;

#[cfg(not(windows))]
impl WindowsClipboardBackend {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(windows))]
impl ClipboardBackend for WindowsClipboardBackend {
    fn set_image(&self, _image: &RgbaImage) -> Result<()> {
        anyhow::bail!("Clipboard not supported on this platform")
    }
    fn get_image(&self) -> Result<Option<RgbaImage>> {
        anyhow::bail!("Clipboard not supported on this platform")
    }
    fn set_text(&self, _text: &str) -> Result<()> {
        anyhow::bail!("Clipboard not supported on this platform")
    }
    fn get_text(&self) -> Result<Option<String>> {
        anyhow::bail!("Clipboard not supported on this platform")
    }
    fn get_content_type(&self) -> Result<ClipboardContent> {
        anyhow::bail!("Clipboard not supported on this platform")
    }
    fn clear(&self) -> Result<()> {
        anyhow::bail!("Clipboard not supported on this platform")
    }
}
