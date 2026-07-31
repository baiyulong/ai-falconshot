use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatingState {
    pub id: String,
    pub image_path: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub transform: TransformState,
    pub opacity: f32,
    pub always_on_top: bool,
    pub mouse_passthrough: bool,
    pub locked_position: bool,
    pub locked_size: bool,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformState {
    pub scale: f32,
    pub rotation: f32,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
}

impl Default for TransformState {
    fn default() -> Self {
        Self {
            scale: 1.0,
            rotation: 0.0,
            flip_horizontal: false,
            flip_vertical: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatingGroup {
    pub id: String,
    pub name: String,
    pub window_ids: Vec<String>,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub name: String,
    pub windows: Vec<FloatingState>,
    pub groups: Vec<FloatingGroup>,
}
