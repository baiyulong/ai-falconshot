pub mod manager;
pub mod types;

pub use manager::{default_bindings, HotkeyManager};
pub use types::*;

use anyhow::Result;

pub trait HotkeyBackend: Send + Sync {
    fn register(&mut self, hotkey: &HotkeyBinding) -> Result<()>;
    fn unregister(&mut self, id: &str) -> Result<()>;
    fn unregister_all(&mut self) -> Result<()>;
    fn is_registered(&self, id: &str) -> bool;
    fn check_conflict(&self, combo: &KeyCombo) -> Option<String>;
}
