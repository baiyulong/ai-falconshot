use crate::types::{FloatingState, TransformState};
use anyhow::Result;
use std::path::Path;

pub trait FloatingWindow: Send {
    fn create(&mut self, image_path: &Path, state: &FloatingState) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn show(&mut self) -> Result<()>;
    fn hide(&mut self) -> Result<()>;
    fn set_transform(&mut self, transform: &TransformState) -> Result<()>;
    fn set_opacity(&mut self, opacity: f32) -> Result<()>;
    fn set_mouse_passthrough(&mut self, enabled: bool) -> Result<()>;
    fn set_always_on_top(&mut self, enabled: bool) -> Result<()>;
    fn get_state(&self) -> &FloatingState;
}
