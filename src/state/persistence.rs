// Save/load workspace layout state to JSON

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Serializable snapshot of the entire app layout.
#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub workspaces: Vec<WorkspaceState>,
    pub active_ws: usize,
    pub sidebar_visible: bool,
    pub show_grid: bool,
    pub show_minimap: bool,
}

/// Serializable snapshot of a single workspace.
#[derive(Serialize, Deserialize)]
pub struct WorkspaceState {
    pub id: String,
    pub name: String,
    pub cwd: Option<PathBuf>,
    pub panels: Vec<PanelState>,
    pub viewport_pan: [f32; 2],
    pub viewport_zoom: f32,
    pub next_z: u32,
    pub next_color: usize,
}

/// Serializable snapshot of a single terminal panel (layout only, no PTY).
#[derive(Serialize, Deserialize)]
pub struct PanelState {
    pub title: String,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [u8; 3],
    pub z_index: u32,
    pub focused: bool,
}

/// Return the path to the state file.
fn state_file_path() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "void")?;
    let data_dir = dirs.data_dir();
    Some(data_dir.join("layout.json"))
}

/// Load saved app state from disk.
pub fn load_state() -> Option<AppState> {
    let path = state_file_path()?;
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Save app state to disk. Creates the directory if needed.
pub fn save_state(state: &AppState) {
    let Some(path) = state_file_path() else {
        log::warn!("Could not determine state file path");
        return;
    };
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            log::warn!("Failed to create state directory: {e}");
            return;
        }
    }
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                log::warn!("Failed to write state file: {e}");
            }
        }
        Err(e) => log::warn!("Failed to serialize state: {e}"),
    }
}
