use tauri::State;

use crate::core::claude_identity::{
    read_claude_fingerprint, regenerate_claude_device_id, restore_claude_device_id,
    ClaudeFingerprint, ClaudeFingerprintSnapshot,
};
use crate::core::direct_claude_route::{
    resolve_proxy_upstream_url, should_follow_cc_switch, sync_direct_claude_route,
};
use crate::core::proxy::{ProxyConfig, ProxyStatus};
use crate::AppState;

const HISTORY_LIMIT: i64 = 200;

#[derive(Debug, serde::Serialize)]
pub struct SwitchAllFingerprintsResult {
    pub fingerprint: ClaudeFingerprintSnapshot,
    pub proxy: ProxyStatus,
}

#[tauri::command]
pub async fn get_claude_fingerprint_snapshot(
    state: State<'_, AppState>,
) -> Result<ClaudeFingerprintSnapshot, String> {
    Ok(read_snapshot(&state))
}

#[tauri::command]
pub async fn regenerate_claude_fingerprint(
    state: State<'_, AppState>,
) -> Result<ClaudeFingerprintSnapshot, String> {
    crate::system::app_log::info("fingerprint.regenerate", "requested");
    let previous = read_claude_fingerprint().map_err(|err| err.to_string())?;
    state
        .db
        .record_claude_fingerprint(&previous, "captured_before_regenerate")
        .map_err(|err| err.to_string())?;

    let updated = regenerate_claude_device_id().map_err(|err| err.to_string())?;
    crate::system::app_log::info(
        "fingerprint.regenerate",
        format!(
            "updated device_id {} -> {}",
            previous
                .device_id
                .as_deref()
                .map(short_device_id)
                .unwrap_or_else(|| "missing".to_string()),
            updated
                .device_id
                .as_deref()
                .map(short_device_id)
                .unwrap_or_else(|| "missing".to_string())
        ),
    );
    state
        .db
        .record_claude_fingerprint(&updated, "generated")
        .map_err(|err| err.to_string())?;

    Ok(read_snapshot(&state))
}

#[tauri::command]
pub async fn switch_all_fingerprints(
    state: State<'_, AppState>,
) -> Result<SwitchAllFingerprintsResult, String> {
    crate::system::app_log::info("fingerprint.switch_all", "requested");

    let previous = read_claude_fingerprint().map_err(|err| err.to_string())?;
    let proxy_status = {
        let profile = state
            .db
            .get_profile()
            .map_err(|err| format!("failed to read profile: {err}"))?;

        let mut proxy = state.proxy.lock().await;
        let current = proxy.status();
        let follow_cc_switch = should_follow_cc_switch(&profile.base_url, 15800);
        let upstream = resolve_proxy_upstream_url(
            &profile.base_url,
            &current.upstream_url,
            follow_cc_switch,
            15800,
        );
        let (target_os, target_arch) = next_target(&current.target_os, &current.target_arch);
        proxy.update_config(ProxyConfig {
            target_os,
            target_arch,
            upstream_url: upstream.upstream_url,
            dynamic_upstream: follow_cc_switch,
        });
        proxy.start().await?;
        let direct_route = sync_direct_claude_route(15800);
        if let Some(error) = direct_route.error.as_deref() {
            crate::system::app_log::error("fingerprint.direct_route", error);
        }
        let mut status = proxy.status();
        status.error = direct_route.error;
        state
            .db
            .save_proxy_target(&status.target_os, &status.target_arch)
            .map_err(|err| format!("failed to save proxy target: {err}"))?;
        status
    };

    state
        .db
        .record_claude_fingerprint(&previous, "captured_before_regenerate")
        .map_err(|err| err.to_string())?;

    let (updated, proxy_status) = regenerate_device_id_after_proxy_start(Ok(proxy_status), || {
        regenerate_claude_device_id().map_err(|err| err.to_string())
    })?;
    state
        .db
        .record_claude_fingerprint(&updated, "generated")
        .map_err(|err| err.to_string())?;

    crate::system::app_log::info(
        "fingerprint.switch_all",
        format!(
            "device_id {} -> {}; proxy {} {} running={}",
            previous
                .device_id
                .as_deref()
                .map(short_device_id)
                .unwrap_or_else(|| "missing".to_string()),
            updated
                .device_id
                .as_deref()
                .map(short_device_id)
                .unwrap_or_else(|| "missing".to_string()),
            proxy_status.target_os,
            proxy_status.target_arch,
            proxy_status.running,
        ),
    );

    Ok(SwitchAllFingerprintsResult {
        fingerprint: read_snapshot(&state),
        proxy: proxy_status,
    })
}

fn regenerate_device_id_after_proxy_start(
    proxy_status: Result<ProxyStatus, String>,
    regenerate: impl FnOnce() -> Result<ClaudeFingerprint, String>,
) -> Result<(ClaudeFingerprint, ProxyStatus), String> {
    let proxy_status = proxy_status?;
    if let Some(error) = proxy_status.error.clone() {
        return Err(error);
    }
    let updated = regenerate()?;
    Ok((updated, proxy_status))
}

#[tauri::command]
pub async fn restore_claude_fingerprint(
    id: String,
    state: State<'_, AppState>,
) -> Result<ClaudeFingerprintSnapshot, String> {
    let history = state
        .db
        .list_claude_fingerprint_history(HISTORY_LIMIT)
        .map_err(|err| err.to_string())?;
    let selected = history
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| "fingerprint history entry not found".to_string())?;
    let device_id = selected
        .device_id
        .as_deref()
        .ok_or_else(|| "fingerprint history entry has no device_id".to_string())?;

    let previous = read_claude_fingerprint().map_err(|err| err.to_string())?;
    state
        .db
        .record_claude_fingerprint(&previous, "captured_before_restore")
        .map_err(|err| err.to_string())?;

    let restored = restore_claude_device_id(device_id).map_err(|err| err.to_string())?;
    state
        .db
        .record_claude_fingerprint(&restored, "restored")
        .map_err(|err| err.to_string())?;

    Ok(read_snapshot(&state))
}

#[tauri::command]
pub async fn delete_claude_fingerprint_history(
    id: String,
    state: State<'_, AppState>,
) -> Result<ClaudeFingerprintSnapshot, String> {
    state
        .db
        .delete_claude_fingerprint_history(&id)
        .map_err(|err| err.to_string())?;
    Ok(read_snapshot(&state))
}

fn read_snapshot(state: &State<'_, AppState>) -> ClaudeFingerprintSnapshot {
    let (current, error) = match read_claude_fingerprint() {
        Ok(fingerprint) => (Some(fingerprint), None),
        Err(err) => (None, Some(err.to_string())),
    };
    let history = state
        .db
        .list_claude_fingerprint_history(HISTORY_LIMIT)
        .unwrap_or_default();

    ClaudeFingerprintSnapshot {
        current,
        history,
        error,
    }
}

fn next_target(current_os: &str, current_arch: &str) -> (String, String) {
    let targets = [
        ("Windows", "x64"),
        ("Windows", "arm64"),
        ("Linux", "x64"),
        ("Linux", "arm64"),
        ("MacOS", "x64"),
        ("MacOS", "arm64"),
    ];
    let mut candidates: Vec<(&str, &str)> = targets
        .into_iter()
        .filter(|(os, arch)| *os != current_os || *arch != current_arch)
        .collect();
    use rand::seq::SliceRandom;
    let (os, arch) = candidates
        .choose_mut(&mut rand::thread_rng())
        .copied()
        .unwrap_or(("Windows", "x64"));
    (os.to_string(), arch.to_string())
}

fn short_device_id(value: &str) -> String {
    if value.len() <= 20 {
        value.to_string()
    } else {
        format!("{}...{}", &value[..8], &value[value.len() - 8..])
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use serde_json::json;
    use tempfile::tempdir;

    use crate::core::claude_identity::regenerate_claude_device_id_at_path;

    use super::*;

    #[test]
    fn proxy_start_failure_does_not_regenerate_device_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".claude.json");
        let original_device_id = "5a60043ffe0e04ff78da4ed3ebebbb4aa9e263b5417f595270b1c646534bf421";
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "theme": "dark",
                "userID": original_device_id,
            }))
            .unwrap(),
        )
        .unwrap();
        let regenerate_called = Cell::new(false);

        let result =
            regenerate_device_id_after_proxy_start(Err("port unavailable".to_string()), || {
                regenerate_called.set(true);
                regenerate_claude_device_id_at_path(&path).map_err(|err| err.to_string())
            });

        assert!(result.is_err());
        assert!(!regenerate_called.get());
        let state: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            state.get("userID").and_then(serde_json::Value::as_str),
            Some(original_device_id)
        );
    }

    #[test]
    fn direct_route_error_does_not_regenerate_device_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".claude.json");
        let original_device_id = "1b750947616d19fc1b8bba20b8024351b1a61012ea72c38b99d33071f4e3a74b";
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "theme": "dark",
                "userID": original_device_id,
            }))
            .unwrap(),
        )
        .unwrap();
        let regenerate_called = Cell::new(false);
        let proxy_status = ProxyStatus {
            running: true,
            listen_port: 15800,
            target_os: "Windows".to_string(),
            target_arch: "x64".to_string(),
            upstream_url: "https://anyrouter.top".to_string(),
            dynamic_upstream: true,
            error: Some("route sync failed".to_string()),
        };

        let result = regenerate_device_id_after_proxy_start(Ok(proxy_status), || {
            regenerate_called.set(true);
            regenerate_claude_device_id_at_path(&path).map_err(|err| err.to_string())
        });

        assert_eq!(result.err().as_deref(), Some("route sync failed"));
        assert!(!regenerate_called.get());
        let state: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            state.get("userID").and_then(serde_json::Value::as_str),
            Some(original_device_id)
        );
    }
}
