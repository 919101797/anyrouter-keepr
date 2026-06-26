use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration as StdDuration;

use chrono::Local;
use tokio::process::Command;
use tokio::time::timeout;

use crate::core::types::ClaudeInstallation;

const VERSION_TIMEOUT_SECONDS: u64 = 5;
const SUMMARY_LIMIT: usize = 300;

pub async fn detect_claude_installation(configured_path: &str) -> ClaudeInstallation {
    let checked_at = Local::now().to_rfc3339();
    let configured_path = configured_path.trim().to_string();

    if !configured_path.is_empty() {
        return detect_manual_path(checked_at, configured_path).await;
    }

    let detected_path = find_binary_in_path("claude").map(path_to_string);
    let Some(effective_path) = detected_path.clone() else {
        return ClaudeInstallation {
            checked_at,
            configured_path,
            detected_path: None,
            effective_path: None,
            version: None,
            source: "path".to_string(),
            status: "not_found".to_string(),
            error: Some("PATH 中没有找到 claude".to_string()),
        };
    };

    installation_from_version_probe(
        checked_at,
        configured_path,
        detected_path,
        effective_path,
        "path",
    )
    .await
}

pub fn configured_or_default_binary(configured_path: &str) -> String {
    let configured_path = configured_path.trim();
    if configured_path.is_empty() {
        "claude".to_string()
    } else {
        configured_path.to_string()
    }
}

async fn detect_manual_path(checked_at: String, configured_path: String) -> ClaudeInstallation {
    let path = resolve_configured_path(&configured_path);
    let Some(effective_path) = path.filter(|path| is_executable_file(path)) else {
        return ClaudeInstallation {
            checked_at,
            configured_path: configured_path.clone(),
            detected_path: None,
            effective_path: Some(configured_path),
            version: None,
            source: "manual".to_string(),
            status: "invalid".to_string(),
            error: Some("手动配置的 claude 路径不存在或不可执行".to_string()),
        };
    };

    installation_from_version_probe(
        checked_at,
        configured_path,
        None,
        path_to_string(effective_path),
        "manual",
    )
    .await
}

async fn installation_from_version_probe(
    checked_at: String,
    configured_path: String,
    detected_path: Option<String>,
    effective_path: String,
    source: &str,
) -> ClaudeInstallation {
    match read_claude_version(&effective_path).await {
        Ok(version) => ClaudeInstallation {
            checked_at,
            configured_path,
            detected_path,
            effective_path: Some(effective_path),
            version: Some(version),
            source: source.to_string(),
            status: "ready".to_string(),
            error: None,
        },
        Err(error) => ClaudeInstallation {
            checked_at,
            configured_path,
            detected_path,
            effective_path: Some(effective_path),
            version: None,
            source: source.to_string(),
            status: "invalid".to_string(),
            error: Some(error),
        },
    }
}

async fn read_claude_version(binary: &str) -> Result<String, String> {
    let child = Command::new(binary)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|err| truncate_summary(&err.to_string()))?;

    let output = timeout(
        StdDuration::from_secs(VERSION_TIMEOUT_SECONDS),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| format!("claude --version 超过 {VERSION_TIMEOUT_SECONDS}s 未返回"))?
    .map_err(|err| truncate_summary(&err.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let summary = [stdout.trim(), stderr.trim()]
        .into_iter()
        .find(|value| !value.is_empty())
        .unwrap_or_default();

    if output.status.success() {
        Ok(truncate_summary(summary))
    } else {
        let code = output
            .status
            .code()
            .map_or_else(|| "unknown".to_string(), |code| code.to_string());
        Err(truncate_summary(&format!(
            "claude --version exited with {code}: {summary}"
        )))
    }
}

fn resolve_configured_path(value: &str) -> Option<PathBuf> {
    let path = PathBuf::from(value);
    if path.is_absolute() || value.contains(std::path::MAIN_SEPARATOR) {
        Some(path)
    } else {
        find_binary_in_path(value).or(Some(path))
    }
}

fn find_binary_in_path(binary_name: &str) -> Option<PathBuf> {
    candidate_dirs()
        .into_iter()
        .flat_map(|dir| candidate_files(&dir, binary_name))
        .find(|path| is_executable_file(path))
}

fn candidate_dirs() -> Vec<PathBuf> {
    let mut seen = HashSet::<OsString>::new();
    let mut dirs = Vec::new();

    if let Some(path) = env::var_os("PATH") {
        for dir in env::split_paths(&path) {
            push_unique_dir(&mut dirs, &mut seen, dir);
        }
    }

    for dir in [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/opt/local/bin",
    ] {
        push_unique_dir(&mut dirs, &mut seen, PathBuf::from(dir));
    }

    if let Some(home) = dirs::home_dir() {
        for relative in [".local/bin", ".npm-global/bin", ".yarn/bin", ".cargo/bin"] {
            push_unique_dir(&mut dirs, &mut seen, home.join(relative));
        }
    }

    dirs
}

fn push_unique_dir(dirs: &mut Vec<PathBuf>, seen: &mut HashSet<OsString>, dir: PathBuf) {
    if seen.insert(dir.clone().into_os_string()) {
        dirs.push(dir);
    }
}

fn candidate_files(dir: &Path, binary_name: &str) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let path_ext = env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_string());
        let mut files = vec![dir.join(binary_name)];
        files.extend(
            path_ext
                .split(';')
                .filter(|ext| !ext.is_empty())
                .map(|ext| dir.join(format!("{binary_name}{ext}"))),
        );
        files
    }

    #[cfg(not(windows))]
    {
        vec![dir.join(binary_name)]
    }
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn truncate_summary(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= SUMMARY_LIMIT {
        return compact;
    }
    let truncated = compact.chars().take(SUMMARY_LIMIT).collect::<String>();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[tokio::test]
    async fn manual_path_reports_version() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("claude");
        fs::write(
            &script,
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '2.1.170 (Claude Code)'; exit 0; fi\n",
        )
        .unwrap();
        #[cfg(unix)]
        make_executable(&script);

        let installation = detect_claude_installation(&path_to_string(script)).await;

        assert_eq!(installation.status, "ready");
        assert_eq!(installation.source, "manual");
        assert_eq!(
            installation.version.as_deref(),
            Some("2.1.170 (Claude Code)")
        );
    }

    #[tokio::test]
    async fn missing_manual_path_is_invalid() {
        let installation = detect_claude_installation("/definitely/missing/claude").await;

        assert_eq!(installation.status, "invalid");
        assert_eq!(installation.source, "manual");
        assert!(installation.error.is_some());
    }

    #[tokio::test]
    async fn live_detects_local_claude_when_enabled() {
        if std::env::var("ANYROUTER_KEEPER_RUN_LIVE_TESTS")
            .ok()
            .as_deref()
            != Some("1")
        {
            return;
        }

        let installation = detect_claude_installation("").await;

        assert_eq!(installation.status, "ready", "{installation:?}");
        assert!(installation.effective_path.is_some(), "{installation:?}");
        assert!(installation.version.is_some(), "{installation:?}");
    }

    #[test]
    fn empty_configured_path_uses_default_binary_name() {
        assert_eq!(configured_or_default_binary(" "), "claude");
        assert_eq!(
            configured_or_default_binary("/usr/local/bin/claude"),
            "/usr/local/bin/claude"
        );
    }
}
