use anyhow::Result;
use floating_core::{FloatingState, FloatingWindow, TransformState};
use std::path::Path;

pub struct WindowsFloatingWindow {
    state: FloatingState,
}

impl WindowsFloatingWindow {
    pub fn new() -> Self {
        Self {
            state: FloatingState {
                id: String::new(),
                image_path: String::new(),
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                transform: TransformState::default(),
                opacity: 1.0,
                always_on_top: true,
                mouse_passthrough: false,
                locked_position: false,
                locked_size: false,
                group_id: None,
            },
        }
    }
}

impl FloatingWindow for WindowsFloatingWindow {
    fn create(&mut self, image_path: &Path, state: &FloatingState) -> Result<()> {
        // TODO: Create Win32 WS_POPUP | WS_EX_TOPMOST | WS_EX_LAYERED window
        self.state = state.clone();
        self.state.image_path = image_path.to_string_lossy().to_string();
        anyhow::bail!("Windows floating window create not yet implemented")
    }

    fn close(&mut self) -> Result<()> {
        // TODO: DestroyWindow
        Ok(())
    }

    fn show(&mut self) -> Result<()> {
        // TODO: ShowWindow(SW_SHOW)
        Ok(())
    }

    fn hide(&mut self) -> Result<()> {
        // TODO: ShowWindow(SW_HIDE)
        Ok(())
    }

    fn set_transform(&mut self, transform: &TransformState) -> Result<()> {
        self.state.transform = transform.clone();
        // TODO: Redraw with Direct2D / WGPU
        Ok(())
    }

    fn set_opacity(&mut self, opacity: f32) -> Result<()> {
        self.state.opacity = opacity;
        // TODO: UpdateLayeredWindow with alpha
        Ok(())
    }

    fn set_mouse_passthrough(&mut self, enabled: bool) -> Result<()> {
        self.state.mouse_passthrough = enabled;
        // TODO: SetWindowLong WS_EX_TRANSPARENT
        Ok(())
    }

    fn set_always_on_top(&mut self, enabled: bool) -> Result<()> {
        self.state.always_on_top = enabled;
        // TODO: SetWindowPos HWND_TOPMOST / HWND_NOTOPMOST
        Ok(())
    }

    fn get_state(&self) -> &FloatingState {
        &self.state
    }
}
