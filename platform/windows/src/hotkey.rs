use anyhow::Result;
use hotkey_core::{HotkeyBackend, HotkeyBinding, KeyCombo};

#[cfg(windows)]
mod win {
    use super::*;
    use hotkey_core::KeyCode;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
        MOD_SHIFT, MOD_WIN,
    };

    pub struct WindowsHotkeyBackend {
        bindings: Vec<HotkeyBinding>,
        hwnd: isize,
        next_id: i32,
    }

    unsafe impl Send for WindowsHotkeyBackend {}

    impl WindowsHotkeyBackend {
        pub fn new() -> Self {
            Self {
                bindings: Vec::new(),
                hwnd: 0,
                next_id: 1,
            }
        }

        pub fn with_hwnd(hwnd: HWND) -> Self {
            Self {
                bindings: Vec::new(),
                hwnd: hwnd.0 as isize,
                next_id: 1,
            }
        }

        fn get_hwnd(&self) -> HWND {
            HWND(self.hwnd as *mut _)
        }

        fn combo_to_modifiers(combo: &KeyCombo) -> HOT_KEY_MODIFIERS {
            let mut mods = MOD_NOREPEAT;
            if combo.ctrl {
                mods |= MOD_CONTROL;
            }
            if combo.alt {
                mods |= MOD_ALT;
            }
            if combo.shift {
                mods |= MOD_SHIFT;
            }
            if combo.win {
                mods |= MOD_WIN;
            }
            mods
        }

        fn key_to_vk(key: &KeyCode) -> u32 {
            match key {
                KeyCode::Char(c) => c.to_ascii_uppercase() as u32,
                KeyCode::F(n) => 0x70 + (*n as u32) - 1,
                KeyCode::Up => 0x26,
                KeyCode::Down => 0x28,
                KeyCode::Left => 0x25,
                KeyCode::Right => 0x27,
                KeyCode::Space => 0x20,
                KeyCode::Enter => 0x0D,
                KeyCode::Escape => 0x1B,
                KeyCode::PrintScreen => 0x2C,
            }
        }

        fn find_hotkey_id(&self, id: &str) -> Option<i32> {
            self.bindings
                .iter()
                .position(|b| b.id == id)
                .map(|i| (i + 1) as i32)
        }
    }

    impl HotkeyBackend for WindowsHotkeyBackend {
        fn register(&mut self, hotkey: &HotkeyBinding) -> Result<()> {
            if self.check_conflict(&hotkey.combo).is_some() {
                anyhow::bail!(
                    "Hotkey combo already registered: {}",
                    hotkey.combo.display()
                );
            }

            let mods = Self::combo_to_modifiers(&hotkey.combo);
            let vk = Self::key_to_vk(&hotkey.combo.key);
            let id = self.next_id;

            unsafe {
                RegisterHotKey(Some(self.get_hwnd()), id, mods, vk)
                    .map_err(|e| anyhow::anyhow!("RegisterHotKey failed: {}", e))?;
            }

            self.next_id += 1;
            self.bindings.push(hotkey.clone());
            Ok(())
        }

        fn unregister(&mut self, id: &str) -> Result<()> {
            if let Some(hotkey_id) = self.find_hotkey_id(id) {
                unsafe {
                    let _ = UnregisterHotKey(Some(self.get_hwnd()), hotkey_id);
                }
                self.bindings.retain(|b| b.id != id);
            }
            Ok(())
        }

        fn unregister_all(&mut self) -> Result<()> {
            unsafe {
                for i in 1..self.next_id {
                    let _ = UnregisterHotKey(Some(self.get_hwnd()), i);
                }
            }
            self.bindings.clear();
            self.next_id = 1;
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
}

#[cfg(windows)]
pub use win::WindowsHotkeyBackend;

#[cfg(not(windows))]
pub struct WindowsHotkeyBackend {
    bindings: Vec<HotkeyBinding>,
}

#[cfg(not(windows))]
impl WindowsHotkeyBackend {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }
}

#[cfg(not(windows))]
impl HotkeyBackend for WindowsHotkeyBackend {
    fn register(&mut self, hotkey: &HotkeyBinding) -> Result<()> {
        self.bindings.push(hotkey.clone());
        Ok(())
    }
    fn unregister(&mut self, id: &str) -> Result<()> {
        self.bindings.retain(|b| b.id != id);
        Ok(())
    }
    fn unregister_all(&mut self) -> Result<()> {
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
