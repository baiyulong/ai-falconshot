use crate::types::{FloatingGroup, FloatingState, TransformState, WorkspaceState};
use crate::window::FloatingWindow;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct FloatingManager<W: FloatingWindow> {
    windows: HashMap<String, W>,
    groups: Vec<FloatingGroup>,
    state_dir: PathBuf,
}

impl<W: FloatingWindow> FloatingManager<W> {
    pub fn new(state_dir: PathBuf) -> Self {
        Self {
            windows: HashMap::new(),
            groups: Vec::new(),
            state_dir,
        }
    }

    pub fn create_window(
        &mut self,
        id: &str,
        image_path: &Path,
        state: &FloatingState,
        mut window: W,
    ) -> Result<()> {
        window.create(image_path, state)?;
        self.windows.insert(id.to_string(), window);
        self.save_state()?;
        Ok(())
    }

    pub fn close_window(&mut self, id: &str) -> Result<()> {
        if let Some(mut window) = self.windows.remove(id) {
            window.close()?;
            for group in &mut self.groups {
                group.window_ids.retain(|wid| wid != id);
            }
            self.save_state()?;
        }
        Ok(())
    }

    pub fn close_all(&mut self) -> Result<()> {
        let ids: Vec<String> = self.windows.keys().cloned().collect();
        for id in ids {
            self.close_window(&id)?;
        }
        Ok(())
    }

    pub fn get_window(&self, id: &str) -> Option<&W> {
        self.windows.get(id)
    }

    pub fn get_window_mut(&mut self, id: &str) -> Option<&mut W> {
        self.windows.get_mut(id)
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn set_opacity(&mut self, id: &str, opacity: f32) -> Result<()> {
        if let Some(window) = self.windows.get_mut(id) {
            window.set_opacity(opacity)?;
            self.save_state()?;
        }
        Ok(())
    }

    pub fn set_all_opacity(&mut self, opacity: f32) -> Result<()> {
        for window in self.windows.values_mut() {
            window.set_opacity(opacity)?;
        }
        self.save_state()?;
        Ok(())
    }

    pub fn toggle_passthrough(&mut self, id: &str) -> Result<()> {
        if let Some(window) = self.windows.get_mut(id) {
            let current = window.get_state().mouse_passthrough;
            window.set_mouse_passthrough(!current)?;
            self.save_state()?;
        }
        Ok(())
    }

    pub fn set_transform(&mut self, id: &str, transform: &TransformState) -> Result<()> {
        if let Some(window) = self.windows.get_mut(id) {
            window.set_transform(transform)?;
            self.save_state()?;
        }
        Ok(())
    }

    pub fn show_all(&mut self) -> Result<()> {
        for window in self.windows.values_mut() {
            window.show()?;
        }
        Ok(())
    }

    pub fn hide_all(&mut self) -> Result<()> {
        for window in self.windows.values_mut() {
            window.hide()?;
        }
        Ok(())
    }

    pub fn create_group(&mut self, id: &str, name: &str, window_ids: Vec<String>) {
        self.groups.push(FloatingGroup {
            id: id.to_string(),
            name: name.to_string(),
            window_ids,
            visible: true,
        });
    }

    pub fn remove_group(&mut self, id: &str) {
        self.groups.retain(|g| g.id != id);
    }

    pub fn groups(&self) -> &[FloatingGroup] {
        &self.groups
    }

    pub fn save_state(&self) -> Result<()> {
        let workspace = self.collect_state();
        let json = serde_json::to_string_pretty(&workspace)?;
        let path = self.state_dir.join("floating_workspace.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub fn load_state(&mut self) -> Result<Option<WorkspaceState>> {
        let path = self.state_dir.join("floating_workspace.json");
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&path)?;
        let state: WorkspaceState = serde_json::from_str(&json)?;
        self.groups = state.groups.clone();
        Ok(Some(state))
    }

    fn collect_state(&self) -> WorkspaceState {
        let windows: Vec<FloatingState> = self
            .windows
            .values()
            .map(|w| w.get_state().clone())
            .collect();
        WorkspaceState {
            name: "default".to_string(),
            windows,
            groups: self.groups.clone(),
        }
    }
}
