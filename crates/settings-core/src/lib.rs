pub mod types;

pub use types::*;

use anyhow::Result;

pub trait SettingsBackend: Send + Sync {
    fn load(&self) -> Result<AppSettings>;
    fn save(&self, settings: &AppSettings) -> Result<()>;
    fn get_value(&self, key: &str) -> Result<Option<serde_json::Value>>;
    fn set_value(&self, key: &str, value: serde_json::Value) -> Result<()>;
    fn reset_defaults(&self) -> Result<()>;
}
