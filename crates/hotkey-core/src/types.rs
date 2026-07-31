use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyBinding {
    pub id: String,
    pub action: HotkeyAction,
    pub combo: KeyCombo,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotkeyAction {
    Capture,
    PinToScreen,
    ToggleAllPins,
    ColorPicker,
    Ocr,
    AiAnalyze,
    QuickSave,
    QuickCopy,
    TogglePassthrough,
    OpenSettings,
    PauseAllHotkeys,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombo {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub key: KeyCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    Char(char),
    F(u8),
    Up,
    Down,
    Left,
    Right,
    Space,
    Enter,
    Escape,
    PrintScreen,
}

impl KeyCombo {
    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.win {
            parts.push("Win");
        }
        parts.push(match self.key {
            KeyCode::Char(c) => return format!("{}+{}", parts.join("+"), c.to_uppercase()),
            KeyCode::F(n) => return format!("{}+F{}", parts.join("+"), n),
            KeyCode::Up => "Up",
            KeyCode::Down => "Down",
            KeyCode::Left => "Left",
            KeyCode::Right => "Right",
            KeyCode::Space => "Space",
            KeyCode::Enter => "Enter",
            KeyCode::Escape => "Esc",
            KeyCode::PrintScreen => "PrtSc",
        });
        parts.join("+")
    }
}
