use crate::types::AppSettings;
use crate::SettingsBackend;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct JsonSettingsBackend {
    path: PathBuf,
    cache: Mutex<Option<AppSettings>>,
}

impl JsonSettingsBackend {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            cache: Mutex::new(None),
        }
    }

    pub fn app_data_dir() -> PathBuf {
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".config"))
                    .unwrap_or_else(|_| PathBuf::from("."))
            });
        base.join("FalconShot")
    }

    pub fn default_path() -> PathBuf {
        Self::app_data_dir().join("settings.json")
    }
}

impl SettingsBackend for JsonSettingsBackend {
    fn load(&self) -> Result<AppSettings> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(ref settings) = *cache {
            return Ok(settings.clone());
        }

        if !self.path.exists() {
            let defaults = AppSettings::default();
            self.save_to_disk(&defaults)?;
            *cache = Some(defaults.clone());
            return Ok(defaults);
        }

        let json = std::fs::read_to_string(&self.path)?;
        let settings: AppSettings = serde_json::from_str(&json)?;
        *cache = Some(settings.clone());
        Ok(settings)
    }

    fn save(&self, settings: &AppSettings) -> Result<()> {
        self.save_to_disk(settings)?;
        let mut cache = self.cache.lock().unwrap();
        *cache = Some(settings.clone());
        Ok(())
    }

    fn get_value(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let settings = self.load()?;
        let json = serde_json::to_value(&settings)?;
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = &json;
        for part in &parts {
            match current.get(part) {
                Some(v) => current = v,
                None => return Ok(None),
            }
        }
        Ok(Some(current.clone()))
    }

    fn set_value(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let mut settings = self.load()?;
        let mut json = serde_json::to_value(&mut settings)?;
        let parts: Vec<&str> = key.split('.').collect();

        let mut current = &mut json;
        for part in &parts[..parts.len() - 1] {
            current = current
                .get_mut(part)
                .ok_or_else(|| anyhow::anyhow!("Key path not found: {key}"))?;
        }
        if let Some(last) = parts.last() {
            current[*last] = value;
        }

        let updated: AppSettings = serde_json::from_value(json)?;
        self.save(&updated)
    }

    fn reset_defaults(&self) -> Result<()> {
        let defaults = AppSettings::default();
        self.save(&defaults)
    }
}

impl JsonSettingsBackend {
    fn save_to_disk(&self, settings: &AppSettings) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(settings)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_settings_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "falconshot_test_settings_{}.json",
            std::process::id()
        ))
    }

    #[test]
    fn test_load_defaults() {
        let path = temp_settings_path();
        let _ = std::fs::remove_file(&path);
        let backend = JsonSettingsBackend::new(path.clone());
        let settings = backend.load().unwrap();
        assert_eq!(settings.general.language, "system");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save_and_load() {
        let path = temp_settings_path();
        let _ = std::fs::remove_file(&path);
        let backend = JsonSettingsBackend::new(path.clone());

        let mut settings = AppSettings::default();
        settings.general.language = "en-US".to_string();
        backend.save(&settings).unwrap();

        let loaded = backend.load().unwrap();
        assert_eq!(loaded.general.language, "en-US");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_get_value() {
        let path = temp_settings_path();
        let _ = std::fs::remove_file(&path);
        let backend = JsonSettingsBackend::new(path.clone());
        let _ = backend.load().unwrap();

        let val = backend.get_value("general.language").unwrap();
        assert_eq!(val, Some(serde_json::json!("system")));

        let missing = backend.get_value("nonexistent.key").unwrap();
        assert_eq!(missing, None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_reset_defaults() {
        let path = temp_settings_path();
        let _ = std::fs::remove_file(&path);
        let backend = JsonSettingsBackend::new(path.clone());

        let mut settings = AppSettings::default();
        settings.general.language = "ja".to_string();
        backend.save(&settings).unwrap();

        backend.reset_defaults().unwrap();
        let loaded = backend.load().unwrap();
        assert_eq!(loaded.general.language, "system");
        let _ = std::fs::remove_file(&path);
    }
}
