use tauri::State;

use crate::core::claude_installation::detect_claude_installation;
use crate::core::types::{Profile, StoredProfile};
use crate::AppState;

#[tauri::command]
pub async fn get_profile(state: State<'_, AppState>) -> Result<StoredProfile, String> {
    state.db.get_profile().map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn save_profile(
    profile: Profile,
    state: State<'_, AppState>,
) -> Result<StoredProfile, String> {
    let previous_path = state
        .db
        .get_profile()
        .ok()
        .map(|profile| profile.claude_binary_path)
        .unwrap_or_default();

    let saved = state
        .db
        .save_profile(profile)
        .map_err(|err| err.to_string())?;

    if previous_path.trim() != saved.claude_binary_path.trim() {
        let installation = detect_claude_installation(&saved.claude_binary_path).await;
        state
            .db
            .record_claude_detection(&installation)
            .map_err(|err| err.to_string())?;
    }

    Ok(saved)
}
