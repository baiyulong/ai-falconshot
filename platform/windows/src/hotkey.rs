use anyhow::Result;
use hotkey_core::{HotkeyBackend, HotkeyBinding, KeyCombo};

pub struct WindowsHotkeyBackend {
    bindings: Vec<HotkeyBinding>,
}

impl WindowsHotkeyBackend {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }
}

impl HotkeyBackend for WindowsHotkeyBackend {
    fn register(&mut self, hotkey: &HotkeyBinding) -> Result<()> {
        // TODO: Win32 RegisterHotKey
        self.bindings.push(hotkey.clone());
        Ok(())
    }

    fn unregister(&mut self, id: &str) -> Result<()> {
        // TODO: Win32 UnregisterHotKey
        self.bindings.retain(|b| b.id != id);
        Ok(())
    }

    fn unregister_all(&mut self) -> Result<()> {
        // TODO: Unregister all hotkeys
        self.bindings.clear();
        Ok(())
    }

    fn is_registered(&self, id: &str) -> bool {
        self.bindings.iter().any(|b| b.id == id)
    }

    fn check_conflict(&self, combo: &KeyCombo) -> Option<String> {
        self.bindings
            .iter()
            .find(|b| &b.combo == combo)
            .map(|b| b.id.clone())
    }
}
