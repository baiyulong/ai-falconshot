pub mod types;

use anyhow::Result;
use image::RgbaImage;
pub use types::ClipboardContent;

pub trait ClipboardBackend: Send + Sync {
    fn set_image(&self, image: &RgbaImage) -> Result<()>;
    fn get_image(&self) -> Result<Option<RgbaImage>>;
    fn set_text(&self, text: &str) -> Result<()>;
    fn get_text(&self) -> Result<Option<String>>;
    fn get_content_type(&self) -> Result<ClipboardContent>;
    fn clear(&self) -> Result<()>;
}
