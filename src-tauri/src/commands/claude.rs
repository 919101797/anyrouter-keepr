use tauri::State;

use crate::core::claude_installation::detect_claude_installation;
use crate::core::claude_runtime_config::{
    detect_claude_runtime_config, resolve_claude_credential, resolve_claude_key_value,
};
use crate::core::direct_claude_route::{resolve_proxy_upstream_url, should_follow_cc_switch};
use crate::core::types::{
    ClaudeDetectionLog, ClaudeInstallation, ClaudeRuntimeConfig, UpstreamModelCatalog,
};
use crate::core::upstream_models::fetch_upstream_models;
use crate::AppState;

const DEFAULT_PROXY_PORT: u16 = 15800;

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

#[tauri::command]
pub async fn get_upstream_models(
    state: State<'_, AppState>,
) -> Result<UpstreamModelCatalog, String> {
    let profile = state
        .db
        .get_runtime_profile()
        .map_err(|err| err.to_string())?;
    let proxy_status = {
        let proxy = state.proxy.lock().await;
        proxy.status()
    };
    let follow_cc_switch = should_follow_cc_switch(&profile.base_url, DEFAULT_PROXY_PORT);
    let upstream = resolve_proxy_upstream_url(
        &profile.base_url,
        &proxy_status.upstream_url,
        follow_cc_switch,
        DEFAULT_PROXY_PORT,
    );
    let credential = resolve_claude_credential(
        None,
        &profile.token_kind,
        profile.token.as_deref().unwrap_or_default(),
    );

    Ok(fetch_upstream_models(
        &upstream.upstream_url,
        &upstream.source,
        credential.as_ref(),
    )
    .await)
}
