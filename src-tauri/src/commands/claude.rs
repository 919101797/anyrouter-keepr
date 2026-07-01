use tauri::State;

use crate::core::claude_installation::detect_claude_installation;
use crate::core::claude_runtime_config::{detect_claude_runtime_config, resolve_claude_key_value};
use crate::core::types::{ClaudeDetectionLog, ClaudeInstallation, ClaudeRuntimeConfig};
use crate::AppState;

#[tauri::command]
pub async fn get_claude_installation(
    state: State<'_, AppState>,
) -> Result<ClaudeInstallation, String> {
    let profile = state.db.get_profile().map_err(|err| err.to_string())?;
    Ok(detect_claude_installation(&profile.claude_binary_path).await)
}

#[tauri::command]
pub async fn refresh_claude_installation(
    state: State<'_, AppState>,
) -> Result<ClaudeInstallation, String> {
    let profile = state.db.get_profile().map_err(|err| err.to_string())?;
    let installation = detect_claude_installation(&profile.claude_binary_path).await;
    state
        .db
        .record_claude_detection(&installation)
        .map_err(|err| err.to_string())?;
    Ok(installation)
}

#[tauri::command]
pub async fn test_claude_installation(
    configured_path: String,
) -> Result<ClaudeInstallation, String> {
    Ok(detect_claude_installation(&configured_path).await)
}

#[tauri::command]
pub async fn list_claude_detection_logs(
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<ClaudeDetectionLog>, String> {
    state
        .db
        .list_claude_detection_logs(limit.unwrap_or(20))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_claude_runtime_config() -> Result<ClaudeRuntimeConfig, String> {
    Ok(detect_claude_runtime_config())
}

#[tauri::command]
pub async fn get_claude_key_value(
    key_summary: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let profile = state
        .db
        .get_runtime_profile()
        .map_err(|err| err.to_string())?;
    Ok(resolve_claude_key_value(
        key_summary.as_deref(),
        &profile.token_kind,
        profile.token.as_deref().unwrap_or_default(),
    ))
}
