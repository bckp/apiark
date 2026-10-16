use crate::storage::state::{self, PersistedState};

#[tauri::command]
pub async fn load_persisted_state() -> Result<PersistedState, String> {
    let path = dirs::home_dir()
        .ok_or("Could not determine home directory")?
        .join(".apiark-internal")
        .join("state.json");
    Ok(state::load_persisted_state(&path))
}

#[tauri::command]
pub async fn save_persisted_state(state: PersistedState) -> Result<(), String> {
    let path = dirs::home_dir()
        .ok_or("Could not determine home directory")?
        .join(".apiark-internal")
        .join("state.json");
    state::save_persisted_state(&path, &state)
}
