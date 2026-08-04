use crate::types::{HotkeyAction, HotkeyBinding, KeyCombo, KeyCode};
use crate::HotkeyBackend;
use anyhow::Result;
use std::collections::HashMap;

pub struct HotkeyManager<B: HotkeyBackend> {
    backend: B,
    bindings: HashMap<String, HotkeyBinding>,
    paused: bool,
}

impl<B: HotkeyBackend> HotkeyManager<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            bindings: HashMap::new(),
            paused: false,
        }
    }

    pub fn register(&mut self, binding: HotkeyBinding) -> Result<()> {
        if let Some(conflict) = self.backend.check_conflict(&binding.combo) {
            if conflict != binding.id {
                anyhow::bail!(
                    "Hotkey {} conflicts with existing binding '{}'",
                    binding.combo.display(),
                    conflict
                );
            }
        }
        self.backend.register(&binding)?;
        self.bindings.insert(binding.id.clone(), binding);
        Ok(())
    }

    pub fn unregister(&mut self, id: &str) -> Result<()> {
        self.backend.unregister(id)?;
        self.bindings.remove(id);
        Ok(())
    }

    pub fn unregister_all(&mut self) -> Result<()> {
        self.backend.unregister_all()?;
        self.bindings.clear();
        Ok(())
    }

    pub fn update_binding(&mut self, binding: HotkeyBinding) -> Result<()> {
        self.backend.unregister(&binding.id)?;
        self.register(binding)
    }

    pub fn get_binding(&self, id: &str) -> Option<&HotkeyBinding> {
        self.bindings.get(id)
    }

    pub fn get_binding_for_action(&self, action: HotkeyAction) -> Option<&HotkeyBinding> {
        self.bindings.values().find(|b| b.action == action)
    }

    pub fn all_bindings(&self) -> Vec<&HotkeyBinding> {
        self.bindings.values().collect()
    }

    pub fn is_registered(&self, id: &str) -> bool {
        self.backend.is_registered(id)
    }

    pub fn check_conflict(&self, combo: &KeyCombo) -> Option<String> {
        self.backend.check_conflict(combo)
    }

    pub fn pause_all(&mut self) -> Result<()> {
        self.backend.unregister_all()?;
        self.paused = true;
        Ok(())
    }

    pub fn resume_all(&mut self) -> Result<()> {
        let bindings: Vec<HotkeyBinding> = self.bindings.values().cloned().collect();
        for binding in bindings {
            if binding.enabled {
                self.backend.register(&binding)?;
            }
        }
        self.paused = false;
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn register_defaults(&mut self) -> Result<()> {
        let defaults = default_bindings();
        for binding in defaults {
            if !self.bindings.contains_key(&binding.id) {
                self.register(binding)?;
            }
        }
        Ok(())
    }

    pub fn save_config(&self) -> Result<String> {
        let bindings: Vec<&HotkeyBinding> = self.bindings.values().collect();
        Ok(serde_json::to_string_pretty(&bindings)?)
    }

    pub fn load_config(&mut self, json: &str) -> Result<()> {
        let bindings: Vec<HotkeyBinding> = serde_json::from_str(json)?;
        self.unregister_all()?;
        for binding in bindings {
            self.register(binding)?;
        }
        Ok(())
    }
}

pub fn default_bindings() -> Vec<HotkeyBinding> {
    vec![
        HotkeyBinding {
            id: "capture".to_string(),
            action: HotkeyAction::Capture,
            combo: KeyCombo {
                ctrl: true,
                alt: true,
                shift: false,
                win: false,
                key: KeyCode::Char('a'),
            },
            enabled: true,
        },
        HotkeyBinding {
            id: "pin_to_screen".to_string(),
            action: HotkeyAction::PinToScreen,
            combo: KeyCombo {
                ctrl: true,
                alt: true,
                shift: false,
                win: false,
                key: KeyCode::Char('p'),
            },
            enabled: true,
        },
        HotkeyBinding {
            id: "color_picker".to_string(),
            action: HotkeyAction::ColorPicker,
            combo: KeyCombo {
                ctrl: true,
                alt: true,
                shift: false,
                win: false,
                key: KeyCode::Char('c'),
            },
            enabled: true,
        },
        HotkeyBinding {
            id: "ocr".to_string(),
            action: HotkeyAction::Ocr,
            combo: KeyCombo {
                ctrl: true,
                alt: true,
                shift: false,
                win: false,
                key: KeyCode::Char('o'),
            },
            enabled: true,
        },
        HotkeyBinding {
            id: "toggle_pins".to_string(),
            action: HotkeyAction::ToggleAllPins,
            combo: KeyCombo {
                ctrl: true,
                alt: true,
                shift: true,
                win: false,
                key: KeyCode::Char('p'),
            },
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBackend {
        registered: Vec<HotkeyBinding>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                registered: Vec::new(),
            }
        }
    }

    impl HotkeyBackend for MockBackend {
        fn register(&mut self, hotkey: &HotkeyBinding) -> Result<()> {
            self.registered.push(hotkey.clone());
            Ok(())
        }
        fn unregister(&mut self, id: &str) -> Result<()> {
            self.registered.retain(|b| b.id != id);
            Ok(())
        }
        fn unregister_all(&mut self) -> Result<()> {
            self.registered.clear();
            Ok(())
        }
        fn is_registered(&self, id: &str) -> bool {
            self.registered.iter().any(|b| b.id == id)
        }
        fn check_conflict(&self, combo: &KeyCombo) -> Option<String> {
            self.registered
                .iter()
                .find(|b| &b.combo == combo)
                .map(|b| b.id.clone())
        }
    }

    #[test]
    fn test_register_and_query() {
        let mut mgr = HotkeyManager::new(MockBackend::new());
        let binding = HotkeyBinding {
            id: "test".to_string(),
            action: HotkeyAction::Capture,
            combo: KeyCombo {
                ctrl: true,
                alt: false,
                shift: false,
                win: false,
                key: KeyCode::Char('x'),
            },
            enabled: true,
        };
        mgr.register(binding).unwrap();
        assert!(mgr.is_registered("test"));
        assert_eq!(mgr.all_bindings().len(), 1);
    }

    #[test]
    fn test_conflict_detection() {
        let mut mgr = HotkeyManager::new(MockBackend::new());
        let combo = KeyCombo {
            ctrl: true,
            alt: false,
            shift: false,
            win: false,
            key: KeyCode::Char('x'),
        };
        mgr.register(HotkeyBinding {
            id: "first".to_string(),
            action: HotkeyAction::Capture,
            combo: combo.clone(),
            enabled: true,
        })
        .unwrap();

        let result = mgr.register(HotkeyBinding {
            id: "second".to_string(),
            action: HotkeyAction::Ocr,
            combo,
            enabled: true,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_pause_resume() {
        let mut mgr = HotkeyManager::new(MockBackend::new());
        mgr.register(HotkeyBinding {
            id: "test".to_string(),
            action: HotkeyAction::Capture,
            combo: KeyCombo {
                ctrl: true,
                alt: false,
                shift: false,
                win: false,
                key: KeyCode::Char('x'),
            },
            enabled: true,
        })
        .unwrap();

        mgr.pause_all().unwrap();
        assert!(mgr.is_paused());
        assert!(!mgr.backend.is_registered("test"));

        mgr.resume_all().unwrap();
        assert!(!mgr.is_paused());
        assert!(mgr.backend.is_registered("test"));
    }

    #[test]
    fn test_save_load_config() {
        let mut mgr = HotkeyManager::new(MockBackend::new());
        mgr.register(HotkeyBinding {
            id: "test".to_string(),
            action: HotkeyAction::Capture,
            combo: KeyCombo {
                ctrl: true,
                alt: true,
                shift: false,
                win: false,
                key: KeyCode::F(1),
            },
            enabled: true,
        })
        .unwrap();

        let json = mgr.save_config().unwrap();
        let mut mgr2 = HotkeyManager::new(MockBackend::new());
        mgr2.load_config(&json).unwrap();
        assert!(mgr2.is_registered("test"));
    }

    #[test]
    fn test_default_bindings() {
        let mut mgr = HotkeyManager::new(MockBackend::new());
        mgr.register_defaults().unwrap();
        assert_eq!(mgr.all_bindings().len(), 5);
    }
}
