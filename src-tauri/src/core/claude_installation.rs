use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration as StdDuration;

use chrono::Local;
use tokio::process::Command;
use tokio::time::timeout;

use crate::core::types::ClaudeInstallation;

const VERSION_TIMEOUT_SECONDS: u64 = 5;
const SHELL_LOOKUP_TIMEOUT_SECONDS: u64 = 4;
const SUMMARY_LIMIT: usize = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeBinaryResolution {
    pub configured_path: String,
    pub detected_path: Option<String>,
    pub effective_path: String,
    pub source: String,
}

pub async fn detect_claude_installation(configured_path: &str) -> ClaudeInstallation {
    let checked_at = Local::now().to_rfc3339();
    let configured_path = configured_path.trim().to_string();

    match resolve_claude_binary(&configured_path).await {
        Ok(resolution) => {
            installation_from_version_probe(
                checked_at,
                resolution.configured_path,
                resolution.detected_path,
                resolution.effective_path,
                &resolution.source,
            )
            .await
        }
        Err(error) => ClaudeInstallation {
            checked_at,
            effective_path: if configured_path.is_empty() {
                None
            } else {
                Some(configured_path.clone())
            },
            configured_path,
            detected_path: None,
            version: None,
            source: if error.is_manual { "manual" } else { "path" }.to_string(),
            status: if error.is_manual {
                "invalid"
            } else {
                "not_found"
            }
            .to_string(),
            error: Some(error.message),
        },
    }
}

pub async fn resolve_claude_binary(
    configured_path: &str,
) -> Result<ClaudeBinaryResolution, ClaudeBinaryResolutionError> {
    let configured_path = configured_path.trim().to_string();

    if configured_path.is_empty() {
        let Some((path, source)) = find_binary("claude").await else {
            return Err(ClaudeBinaryResolutionError::auto_not_found());
        };
        let effective_path = path_to_string(path);
        return Ok(ClaudeBinaryResolution {
            configured_path,
            detected_path: Some(effective_path.clone()),
            effective_path,
            source,
        });
    }

    let Some((path, source)) = resolve_configured_path(&configured_path).await else {
        return Err(ClaudeBinaryResolutionError::manual_invalid(configured_path));
    };

    Ok(ClaudeBinaryResolution {
        configured_path,
        detected_path: None,
        effective_path: path_to_string(path),
        source,
    })
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
    let mut command = Command::new(binary);
    apply_claude_command_path(&mut command, binary);
    let child = command
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

pub fn apply_claude_command_path(command: &mut Command, binary: &str) {
    command.env("PATH", command_path(binary));
}

fn command_path(binary: &str) -> OsString {
    let mut seen = HashSet::<OsString>::new();
    let mut dirs = Vec::new();

    if let Some(parent) = Path::new(binary)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        push_unique_dir(&mut dirs, &mut seen, parent.to_path_buf());
    }

    for dir in candidate_dirs() {
        push_unique_dir(&mut dirs, &mut seen, dir);
    }

    env::join_paths(&dirs).unwrap_or_else(|_| env::var_os("PATH").unwrap_or_default())
}

async fn resolve_configured_path(value: &str) -> Option<(PathBuf, String)> {
    let path = PathBuf::from(value);
    if path.is_absolute() || value.contains(path::MAIN_SEPARATOR) {
        if is_executable_file(&path) {
            Some((path, "manual".to_string()))
        } else {
            None
        }
    } else if let Some(path) = find_binary_in_path(value) {
        Some((path, "manual".to_string()))
    } else {
        find_binary_in_user_shell(value)
            .await
            .map(|path| (path, "manual".to_string()))
    }
}

async fn find_binary(binary_name: &str) -> Option<(PathBuf, String)> {
    if let Some(path) = find_binary_in_path(binary_name) {
        return Some((path, "path".to_string()));
    }

    find_binary_in_user_shell(binary_name)
        .await
        .map(|path| (path, "shell".to_string()))
}

fn find_binary_in_path(binary_name: &str) -> Option<PathBuf> {
    find_binary_in_dirs(binary_name, candidate_dirs())
}

fn find_binary_in_dirs(binary_name: &str, dirs: Vec<PathBuf>) -> Option<PathBuf> {
    dirs.into_iter()
        .flat_map(|dir| candidate_files(&dir, binary_name))
        .find(|path| is_executable_file(path))
}

#[cfg(test)]
fn resolve_configured_path_in_dirs(value: &str, dirs: Vec<PathBuf>) -> Option<PathBuf> {
    let path = PathBuf::from(value);
    if path.is_absolute() || value.contains(path::MAIN_SEPARATOR) {
        is_executable_file(&path).then_some(path)
    } else {
        find_binary_in_dirs(value, dirs)
    }
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
        for relative in [
            ".local/bin",
            ".npm-global/bin",
            ".npm-packages/bin",
            ".yarn/bin",
            ".cargo/bin",
            ".volta/bin",
            ".asdf/shims",
            ".mise/shims",
            ".nodenv/shims",
            ".bun/bin",
            ".local/share/pnpm",
            "Library/pnpm",
            ".local/share/mise/shims",
        ] {
            push_unique_dir(&mut dirs, &mut seen, home.join(relative));
        }
        push_child_bin_dirs(&mut dirs, &mut seen, &home.join(".nvm/versions/node"));
        push_fnm_bin_dirs(&mut dirs, &mut seen, &home.join(".fnm/node-versions"));
    }

    dirs
}

fn push_child_bin_dirs(dirs: &mut Vec<PathBuf>, seen: &mut HashSet<OsString>, root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut children = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("bin"))
        .collect::<Vec<_>>();
    children.sort();
    children.reverse();
    for dir in children {
        push_unique_dir(dirs, seen, dir);
    }
}

fn push_fnm_bin_dirs(dirs: &mut Vec<PathBuf>, seen: &mut HashSet<OsString>, root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut children = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("installation/bin"))
        .collect::<Vec<_>>();
    children.sort();
    children.reverse();
    for dir in children {
        push_unique_dir(dirs, seen, dir);
    }
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

#[cfg(unix)]
async fn find_binary_in_user_shell(binary_name: &str) -> Option<PathBuf> {
    if !is_plain_binary_name(binary_name) {
        return None;
    }

    let shell = env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| is_executable_file(path))
        .unwrap_or_else(default_shell);

    let child = Command::new(shell)
        .arg("-lic")
        .arg(format!("command -v {binary_name}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .ok()?;

    let output = timeout(
        StdDuration::from_secs(SHELL_LOOKUP_TIMEOUT_SECONDS),
        child.wait_with_output(),
    )
    .await
    .ok()?
    .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().find_map(|line| {
        let trimmed = line.trim();
        let path = PathBuf::from(trimmed);
        if path.is_absolute() && is_executable_file(&path) {
            Some(path)
        } else {
            None
        }
    })
}

#[cfg(not(unix))]
async fn find_binary_in_user_shell(_binary_name: &str) -> Option<PathBuf> {
    None
}

fn is_plain_binary_name(binary_name: &str) -> bool {
    !binary_name.is_empty()
        && binary_name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

#[cfg(all(unix, target_os = "macos"))]
fn default_shell() -> PathBuf {
    PathBuf::from("/bin/zsh")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_shell() -> PathBuf {
    PathBuf::from("/bin/sh")
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeBinaryResolutionError {
    pub message: String,
    pub is_manual: bool,
}

impl ClaudeBinaryResolutionError {
    fn auto_not_found() -> Self {
        Self {
            message: "未找到 Claude Code CLI。macOS 打包 App 可能拿不到终端 PATH，请在 Claude 可执行文件路径里填入 `command -v claude` 输出的绝对路径，例如 /opt/homebrew/bin/claude、/usr/local/bin/claude 或你的 nvm/volta/asdf 安装路径。".to_string(),
            is_manual: false,
        }
    }

    fn manual_invalid(configured_path: String) -> Self {
        Self {
            message: format!(
                "手动配置的 Claude Code CLI 路径不存在或不可执行：{configured_path}。请填入 `command -v claude` 输出的绝对路径。"
            ),
            is_manual: true,
        }
    }
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
    fn finds_executable_binary_from_candidate_dirs() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("claude");
        fs::write(&script, "#!/bin/sh\necho ok\n").unwrap();
        #[cfg(unix)]
        make_executable(&script);

        let found = find_binary_in_dirs("claude", vec![dir.path().to_path_buf()]);

        assert_eq!(found.as_deref(), Some(script.as_path()));
    }

    #[test]
    fn resolves_manual_binary_name_through_candidate_dirs() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("claude-test");
        fs::write(&script, "#!/bin/sh\necho ok\n").unwrap();
        #[cfg(unix)]
        make_executable(&script);

        let resolved =
            resolve_configured_path_in_dirs("claude-test", vec![dir.path().to_path_buf()]);

        assert_eq!(resolved.as_deref(), Some(script.as_path()));
    }

    #[test]
    fn command_path_puts_binary_parent_first() {
        let dir = tempdir().unwrap();
        let binary = dir.path().join("claude");

        let path_value = command_path(binary.to_str().unwrap());
        let mut paths = env::split_paths(&path_value);

        assert_eq!(paths.next().as_deref(), Some(dir.path()));
    }
}
