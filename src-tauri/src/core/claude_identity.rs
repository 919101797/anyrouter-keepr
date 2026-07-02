use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClaudeIdentityError {
    #[error("Claude state file not found: {0}")]
    MissingState(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Claude state root must be a JSON object")]
    InvalidStateRoot,
    #[error("device_id must be a 64 character hex string")]
    InvalidDeviceId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaudeFingerprint {
    pub checked_at: String,
    pub claude_state_path: String,
    pub stainless_os: String,
    pub stainless_arch: String,
    pub device_id: Option<String>,
    pub device_id_status: String,
    pub session_id_status: String,
    pub risk_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaudeFingerprintHistoryEntry {
    pub id: String,
    pub captured_at: String,
    pub source: String,
    pub claude_state_path: String,
    pub stainless_os: String,
    pub stainless_arch: String,
    pub device_id: Option<String>,
    pub device_id_status: String,
    pub session_id_status: String,
    pub risk_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaudeFingerprintSnapshot {
    pub current: Option<ClaudeFingerprint>,
    pub history: Vec<ClaudeFingerprintHistoryEntry>,
    pub error: Option<String>,
}

pub fn read_claude_fingerprint() -> Result<ClaudeFingerprint, ClaudeIdentityError> {
    read_claude_fingerprint_from_path(&default_claude_state_path()?)
}

pub fn regenerate_claude_device_id() -> Result<ClaudeFingerprint, ClaudeIdentityError> {
    regenerate_claude_device_id_at_path(&default_claude_state_path()?)
}

pub fn read_claude_fingerprint_from_path(
    path: &Path,
) -> Result<ClaudeFingerprint, ClaudeIdentityError> {
    let value = read_state_json_or_empty(path)?;
    Ok(fingerprint_from_state(path, &value))
}

pub fn regenerate_claude_device_id_at_path(
    path: &Path,
) -> Result<ClaudeFingerprint, ClaudeIdentityError> {
    write_claude_device_id_at_path(path, &generate_device_id())
}

pub fn restore_claude_device_id_at_path(
    path: &Path,
    device_id: &str,
) -> Result<ClaudeFingerprint, ClaudeIdentityError> {
    if !is_valid_device_id(device_id) {
        return Err(ClaudeIdentityError::InvalidDeviceId);
    }
    write_claude_device_id_at_path(path, device_id)
}

pub fn restore_claude_device_id(device_id: &str) -> Result<ClaudeFingerprint, ClaudeIdentityError> {
    restore_claude_device_id_at_path(&default_claude_state_path()?, device_id)
}

fn write_claude_device_id_at_path(
    path: &Path,
    device_id: &str,
) -> Result<ClaudeFingerprint, ClaudeIdentityError> {
    let mut value = read_state_json_or_empty(path)?;
    let object = value
        .as_object_mut()
        .ok_or(ClaudeIdentityError::InvalidStateRoot)?;
    object.insert("userID".to_string(), Value::String(device_id.to_string()));
    fs::write(path, serde_json::to_string_pretty(&value)?)?;
    Ok(fingerprint_from_state(path, &value))
}

pub fn default_claude_state_path() -> Result<PathBuf, ClaudeIdentityError> {
    let home = dirs::home_dir().ok_or_else(|| {
        ClaudeIdentityError::MissingState("home directory unavailable".to_string())
    })?;
    Ok(home.join(".claude.json"))
}

pub fn current_stainless_os() -> String {
    match std::env::consts::OS {
        "macos" => "MacOS",
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    }
    .to_string()
}

pub fn current_stainless_arch() -> String {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        other => other,
    }
    .to_string()
}

fn read_state_json_or_empty(path: &Path) -> Result<Value, ClaudeIdentityError> {
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn fingerprint_from_state(path: &Path, value: &Value) -> ClaudeFingerprint {
    let device_id = value
        .get("userID")
        .and_then(Value::as_str)
        .map(str::to_string);
    ClaudeFingerprint {
        checked_at: Local::now().to_rfc3339(),
        claude_state_path: path.to_string_lossy().into_owned(),
        stainless_os: current_stainless_os(),
        stainless_arch: current_stainless_arch(),
        device_id_status: device_id_status(device_id.as_deref()).to_string(),
        device_id,
        session_id_status: "runtime_generated_by_claude_code".to_string(),
        risk_label: "AnyRouter 1M routing key: OS + Arch + device_id".to_string(),
    }
}

fn device_id_status(device_id: Option<&str>) -> &'static str {
    match device_id {
        Some(value) if is_valid_device_id(value) => "present",
        Some(_) => "invalid_format",
        None => "missing",
    }
}

fn is_valid_device_id(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|char| char.is_ascii_hexdigit())
}

fn generate_device_id() -> String {
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn reads_device_id_from_claude_state_user_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".claude.json");
        let device_id = "1b750947616d19fc1b8bba20b8024351b1a61012ea72c38b99d33071f4e3a74b";
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "theme": "dark",
                "userID": device_id,
                "numStartups": 42
            }))
            .unwrap(),
        )
        .unwrap();

        let fingerprint = read_claude_fingerprint_from_path(&path).unwrap();

        assert_eq!(fingerprint.device_id.as_deref(), Some(device_id));
        assert_eq!(fingerprint.device_id_status, "present");
        assert_eq!(fingerprint.stainless_os, current_stainless_os());
        assert_eq!(fingerprint.stainless_arch, current_stainless_arch());
        assert_eq!(fingerprint.claude_state_path, path.to_string_lossy());
    }

    #[test]
    fn regenerates_device_id_as_lowercase_64_hex_and_preserves_state() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".claude.json");
        let old_device_id = "5a60043ffe0e04ff78da4ed3ebebbb4aa9e263b5417f595270b1c646534bf421";
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "theme": "dark",
                "userID": old_device_id,
                "numStartups": 42
            }))
            .unwrap(),
        )
        .unwrap();

        let updated = regenerate_claude_device_id_at_path(&path).unwrap();
        let state = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&state).unwrap();
        let new_device_id = parsed
            .get("userID")
            .and_then(serde_json::Value::as_str)
            .unwrap();

        assert_ne!(new_device_id, old_device_id);
        assert_eq!(new_device_id.len(), 64);
        assert!(new_device_id
            .chars()
            .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase()));
        assert_eq!(
            parsed.get("theme").and_then(serde_json::Value::as_str),
            Some("dark")
        );
        assert_eq!(
            parsed
                .get("numStartups")
                .and_then(serde_json::Value::as_i64),
            Some(42)
        );
        assert_eq!(updated.device_id.as_deref(), Some(new_device_id));
    }

    #[test]
    fn restores_device_id_and_preserves_state() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".claude.json");
        let active_device_id = "cff2afa3147b17a437d6a02b8d1e83d33bcc619e52d43172fdf3e15b03b6d2e8";
        let restored_device_id = "1b750947616d19fc1b8bba20b8024351b1a61012ea72c38b99d33071f4e3a74b";
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "theme": "dark",
                "userID": active_device_id,
                "numStartups": 42
            }))
            .unwrap(),
        )
        .unwrap();

        let restored = restore_claude_device_id_at_path(&path, restored_device_id).unwrap();
        let state = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&state).unwrap();

        assert_eq!(
            parsed.get("userID").and_then(serde_json::Value::as_str),
            Some(restored_device_id)
        );
        assert_eq!(
            parsed.get("theme").and_then(serde_json::Value::as_str),
            Some("dark")
        );
        assert_eq!(
            parsed
                .get("numStartups")
                .and_then(serde_json::Value::as_i64),
            Some(42)
        );
        assert_eq!(restored.device_id.as_deref(), Some(restored_device_id));
    }
}
