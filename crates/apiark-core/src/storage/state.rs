use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedTab {
    pub file_path: String,
    pub collection_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 100.0,
            y: 100.0,
            width: 1280.0,
            height: 800.0,
            maximized: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistedState {
    pub tabs: Vec<PersistedTab>,
    pub active_tab_index: Option<usize>,
    #[serde(default)]
    pub window_state: Option<WindowState>,
    #[serde(default)]
    pub collections: Vec<String>,
}

pub fn load_persisted_state(path: &Path) -> PersistedState {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse persisted state: {e}");
            PersistedState::default()
        }),
        Err(_) => PersistedState::default(),
    }
}

pub fn save_persisted_state(path: &Path, state: &PersistedState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create state directory: {e}"))?;
    }

    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize state: {e}"))?;

    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &json).map_err(|e| format!("Failed to write state: {e}"))?;
    std::fs::rename(&tmp_path, path).map_err(|e| format!("Failed to rename state file: {e}"))?;

    Ok(())
}
