use anyhow::Result;
use clipboard_core::{ClipboardBackend, ClipboardContent};
use image::RgbaImage;

pub struct WindowsClipboardBackend;

impl WindowsClipboardBackend {
    pub fn new() -> Self {
        Self
    }
}

impl ClipboardBackend for WindowsClipboardBackend {
    fn set_image(&self, _image: &RgbaImage) -> Result<()> {
        // TODO: Implement Win32 SetClipboardData with CF_DIB
        anyhow::bail!("Windows clipboard set_image not yet implemented")
    }

    fn get_image(&self) -> Result<Option<RgbaImage>> {
        // TODO: Implement Win32 GetClipboardData CF_DIB
        Ok(None)
    }

    fn set_text(&self, _text: &str) -> Result<()> {
        // TODO: Implement Win32 SetClipboardData CF_UNICODETEXT
        anyhow::bail!("Windows clipboard set_text not yet implemented")
    }

    fn get_text(&self) -> Result<Option<String>> {
        // TODO: Implement Win32 GetClipboardData CF_UNICODETEXT
        Ok(None)
    }

    fn get_content_type(&self) -> Result<ClipboardContent> {
        // TODO: Implement Win32 EnumClipboardFormats
        Ok(ClipboardContent::Empty)
    }

    fn clear(&self) -> Result<()> {
        // TODO: Implement Win32 EmptyClipboard
        Ok(())
    }
}
