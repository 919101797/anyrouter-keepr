use tauri::State;

use crate::core::types::{ActivityBucket, ProbeEventDto};
use crate::AppState;

#[tauri::command]
pub async fn list_probe_events(
    limit: Option<i64>,
    status: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ProbeEventDto>, String> {
    state
        .db
        .list_events(limit.unwrap_or(200).clamp(1, 2000), status)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_activity_summary(
    hours: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<ActivityBucket>, String> {
    state
        .db
        .activity_summary(hours.unwrap_or(24).clamp(1, 168))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn compact_storage(state: State<'_, AppState>) -> Result<(), String> {
    state.db.compact_storage().map_err(|err| err.to_string())
}
