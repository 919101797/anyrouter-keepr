use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration as StdDuration;

use chrono::Local;
use rand::seq::SliceRandom;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};
use tokio::time::sleep;
use uuid::Uuid;

use crate::core::classifier::classify;
use crate::core::claude_installation::{apply_claude_command_options, resolve_claude_binary};
use crate::core::claude_runtime_config::{
    detect_claude_key_summary, detect_claude_runtime_config, summarize_configured_key,
};
use crate::core::redactor::summarize_and_redact;
use crate::core::types::{ProbeEvent, ProbeStatus, Profile};
use crate::system::app_log;

const FALLBACK_PROMPT: &str = "只回复 OK";
const EMPTY_MCP_CONFIG: &[u8] = br#"{"mcpServers":{}}"#;
const MAX_PROMPT_CHARS: usize = 1_000;
const MAX_PROMPT_SUMMARY_BYTES: usize = 1_024;

pub async fn run_probe(profile: &Profile) -> ProbeEvent {
    match resolve_claude_binary(&profile.claude_binary_path).await {
        Ok(resolution) => {
            app_log::info(
                "run_probe.resolve_claude",
                format!(
                    "source={} effective_path={}",
                    resolution.source, resolution.effective_path
                ),
            );
            run_probe_with_binary(profile, &resolution.effective_path).await
        }
        Err(error) => {
            app_log::error("run_probe.resolve_claude", &error.message);
            claude_not_found_event(profile, &error.message)
        }
    }
}

pub fn claude_not_found_event(profile: &Profile, message: &str) -> ProbeEvent {
    let started_at = Local::now();
    let token = profile.token.clone().unwrap_or_default();
    let key_summary = effective_key_summary(profile, token.trim());
    let effective_model = effective_model(profile);
    let probe_prompt = select_probe_prompt(profile);
    config_error_event(
        profile,
        started_at,
        effective_model.as_deref(),
        key_summary,
        &probe_prompt,
        "claude_not_found",
        message,
    )
}

async fn run_probe_with_binary(profile: &Profile, binary: &str) -> ProbeEvent {
    let started_at = Local::now();
    let token = profile.token.clone().unwrap_or_default();
    let token = token.trim();
    let key_summary = effective_key_summary(profile, token);
    let effective_model = effective_model(profile);
    let probe_prompt = select_probe_prompt(profile);
    app_log::info(
        "run_probe.spawn",
        format!(
            "binary={binary} model={} prompt_chars={}",
            event_model(effective_model.as_deref()),
            probe_prompt.chars().count()
        ),
    );

    if !token.is_empty() && !is_supported_token_kind(&profile.token_kind) {
        return config_error_event(
            profile,
            started_at,
            effective_model.as_deref(),
            key_summary,
            &probe_prompt,
            "invalid_token_kind",
            "token kind must be ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY",
        );
    }

    let mut probe_command = match build_command(profile, binary, token, effective_model.as_deref())
    {
        Ok(command) => command,
        Err(err) => {
            app_log::error("run_probe.build_command", err.to_string());
            return config_error_event(
                profile,
                started_at,
                effective_model.as_deref(),
                key_summary,
                &probe_prompt,
                "mcp_config_error",
                &err.to_string(),
            );
        }
    };

    let mut child = match probe_command.spawn() {
        Ok(child) => child,
        Err(err) => {
            app_log::error("run_probe.spawn", err.to_string());
            let ended_at = Local::now();
            let (prompt_summary, prompt_truncated) = prompt_summary(&probe_prompt);
            return ProbeEvent {
                id: Uuid::new_v4().to_string(),
                profile_id: profile.id.clone(),
                started_at,
                ended_at,
                duration_ms: (ended_at - started_at).num_milliseconds(),
                status: ProbeStatus::ConfigError,
                error_kind: Some("claude_not_found".to_string()),
                exit_code: None,
                base_url: profile.base_url.clone(),
                model: event_model(effective_model.as_deref()),
                key_summary,
                prompt_summary,
                prompt_truncated,
                stdout_summary: None,
                stderr_summary: Some(err.to_string()),
                stdout_truncated: false,
                stderr_truncated: false,
            };
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let prompt = probe_prompt.clone();
        tokio::spawn(async move {
            let _ = stdin.write_all(prompt.as_bytes()).await;
        });
    }

    let timeout_duration = StdDuration::from_secs(profile.timeout_seconds.max(1));
    let output_result = collect_probe_output(&mut child, timeout_duration).await;
    let ended_at = Local::now();

    match output_result {
        Ok(output) => event_from_output(
            profile,
            started_at,
            ended_at,
            OutputCapture {
                exit_code: output.exit_code,
                timed_out: output.timed_out,
                effective_model: effective_model.as_deref(),
                key_summary,
                prompt: &probe_prompt,
                stdout: &output.stdout,
                stderr: &output.stderr,
            },
        ),
        Err(err) => {
            let stderr = err.to_string();
            event_from_output(
                profile,
                started_at,
                ended_at,
                OutputCapture {
                    exit_code: None,
                    timed_out: false,
                    effective_model: effective_model.as_deref(),
                    key_summary,
                    prompt: &probe_prompt,
                    stdout: b"",
                    stderr: stderr.as_bytes(),
                },
            )
        }
    }
}

struct ProbeOutput {
    exit_code: Option<i32>,
    timed_out: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

async fn collect_probe_output(
    child: &mut Child,
    timeout_duration: StdDuration,
) -> Result<ProbeOutput, std::io::Error> {
    let Some(stdout) = child.stdout.take() else {
        let status = child.wait().await?;
        return Ok(ProbeOutput {
            exit_code: status.code(),
            timed_out: false,
            stdout: Vec::new(),
            stderr: Vec::new(),
        });
    };
    let stderr = child.stderr.take();
    let stderr_handle = tokio::spawn(async move {
        let mut buffer = Vec::new();
        if let Some(mut stderr) = stderr {
            let _ = stderr.read_to_end(&mut buffer).await;
        }
        buffer
    });

    let stream_result = collect_stdout_until_done_or_timeout(child, stdout, timeout_duration).await;
    let stderr = stderr_handle.await.unwrap_or_default();
    stream_result.map(|mut output| {
        output.stderr = stderr;
        output
    })
}

async fn collect_stdout_until_done_or_timeout(
    child: &mut Child,
    stdout: ChildStdout,
    timeout_duration: StdDuration,
) -> Result<ProbeOutput, std::io::Error> {
    let mut lines = BufReader::new(stdout).lines();
    let mut stdout = Vec::new();
    let timeout_sleep = sleep(timeout_duration);
    tokio::pin!(timeout_sleep);

    loop {
        tokio::select! {
            line = lines.next_line() => {
                match line? {
                    Some(line) => {
                        stdout.extend_from_slice(line.as_bytes());
                        stdout.push(b'\n');
                    }
                    None => {
                        let status = child.wait().await?;
                        return Ok(ProbeOutput {
                            exit_code: status.code(),
                            timed_out: false,
                            stdout,
                            stderr: Vec::new(),
                        });
                    }
                }
            }
            _ = &mut timeout_sleep => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                let message = format!("probe timed out after {}s", timeout_duration.as_secs());
                return Ok(ProbeOutput {
                    exit_code: None,
                    timed_out: true,
                    stdout,
                    stderr: message.into_bytes(),
                });
            }
        }
    }
}

fn select_probe_prompt(profile: &Profile) -> String {
    let candidates = prompt_pool_candidates(&profile.prompt_pool);
    candidates
        .choose(&mut rand::thread_rng())
        .cloned()
        .unwrap_or_else(|| fallback_prompt(&profile.prompt))
}

fn prompt_pool_candidates(pool: &[String]) -> Vec<String> {
    pool.iter()
        .filter_map(|prompt| {
            let trimmed = prompt.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trim_to_chars(trimmed, MAX_PROMPT_CHARS))
            }
        })
        .collect()
}

fn fallback_prompt(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        FALLBACK_PROMPT.to_string()
    } else {
        trim_to_chars(trimmed, MAX_PROMPT_CHARS)
    }
}

fn trim_to_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn build_command(
    profile: &Profile,
    binary: &str,
    token: &str,
    effective_model: Option<&str>,
) -> io::Result<ProbeCommand> {
    let mut command = Command::new(binary);
    apply_claude_command_options(&mut command, binary);
    let mut temp_files = Vec::new();
    command
        .arg("-p")
        .arg("--safe-mode")
        .arg("--disable-slash-commands")
        .arg("--strict-mcp-config");
    add_empty_mcp_config_arg(&mut command, &mut temp_files)?;
    command
        .arg("--no-session-persistence")
        .arg("--tools")
        .arg("")
        .arg("--output-format")
        .arg("stream-json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .env("CLAUDE_CODE_SKIP_PROMPT_HISTORY", "1");

    if let Some(model) = effective_model {
        command.arg("--model").arg(model);
    }
    if let Some(effort) = normalized_effort(&profile.effort) {
        command.arg("--effort").arg(effort);
    }
    if !profile.base_url.trim().is_empty() {
        command.env("ANTHROPIC_BASE_URL", profile.base_url.trim());
    }
    if !token.is_empty() {
        command.env(&profile.token_kind, token);
    }

    Ok(ProbeCommand {
        command,
        temp_files,
    })
}

struct ProbeCommand {
    command: Command,
    temp_files: Vec<PathBuf>,
}

impl ProbeCommand {
    fn spawn(&mut self) -> io::Result<Child> {
        self.command.spawn()
    }
}

impl Drop for ProbeCommand {
    fn drop(&mut self) {
        for path in &self.temp_files {
            let _ = fs::remove_file(path);
        }
    }
}

fn add_empty_mcp_config_arg(
    command: &mut Command,
    temp_files: &mut Vec<PathBuf>,
) -> io::Result<()> {
    #[cfg(not(windows))]
    let _ = temp_files;

    command.arg("--mcp-config");

    #[cfg(windows)]
    {
        let path = std::env::temp_dir().join(format!(
            "anyrouter-keeper-empty-mcp-{}.json",
            Uuid::new_v4()
        ));
        fs::write(&path, EMPTY_MCP_CONFIG)?;
        command.arg(&path);
        temp_files.push(path);
    }

    #[cfg(not(windows))]
    {
        command.arg(std::str::from_utf8(EMPTY_MCP_CONFIG).expect("static JSON is UTF-8"));
    }

    Ok(())
}

fn is_supported_token_kind(value: &str) -> bool {
    matches!(value.trim(), "ANTHROPIC_AUTH_TOKEN" | "ANTHROPIC_API_KEY")
}

fn effective_model(profile: &Profile) -> Option<String> {
    let detected_model = if profile.model.trim().is_empty() {
        detect_claude_runtime_config().default_model
    } else {
        None
    };
    effective_model_with_default(profile, detected_model.as_deref())
}

fn effective_model_with_default(
    profile: &Profile,
    detected_default_model: Option<&str>,
) -> Option<String> {
    let model = if profile.model.trim().is_empty() {
        detected_default_model.unwrap_or_default()
    } else {
        profile.model.trim()
    };
    if model.is_empty() {
        return None;
    }

    if should_append_context_suffix(model, &profile.context_size) {
        Some(format!("{model}[1m]"))
    } else {
        Some(model.to_string())
    }
}

fn event_model(effective_model: Option<&str>) -> String {
    effective_model.unwrap_or("default").to_string()
}

fn effective_key_summary(profile: &Profile, token: &str) -> Option<String> {
    summarize_configured_key(&profile.token_kind, token).or_else(detect_claude_key_summary)
}

fn prompt_summary(prompt: &str) -> (Option<String>, bool) {
    summarize_and_redact(prompt.as_bytes(), MAX_PROMPT_SUMMARY_BYTES)
}

fn should_append_context_suffix(model: &str, context_size: &str) -> bool {
    context_size.trim().eq_ignore_ascii_case("1m")
        && !model.contains('[')
        && !is_known_non_1m_model(model)
}

fn is_known_non_1m_model(model: &str) -> bool {
    let value = model.trim().to_ascii_lowercase();
    matches!(value.as_str(), "haiku" | "opusplan") || value.starts_with("claude-haiku")
}

fn normalized_effort(value: &str) -> Option<&str> {
    let effort = value.trim();
    if matches!(effort, "low" | "medium" | "high" | "xhigh" | "max") {
        Some(effort)
    } else {
        None
    }
}

fn config_error_event(
    profile: &Profile,
    started_at: chrono::DateTime<Local>,
    effective_model: Option<&str>,
    key_summary: Option<String>,
    prompt: &str,
    kind: &str,
    message: &str,
) -> ProbeEvent {
    let ended_at = Local::now();
    let (prompt_summary, prompt_truncated) = prompt_summary(prompt);
    ProbeEvent {
        id: Uuid::new_v4().to_string(),
        profile_id: profile.id.clone(),
        started_at,
        ended_at,
        duration_ms: (ended_at - started_at).num_milliseconds(),
        status: ProbeStatus::ConfigError,
        error_kind: Some(kind.to_string()),
        exit_code: None,
        base_url: profile.base_url.clone(),
        model: event_model(effective_model),
        key_summary,
        prompt_summary,
        prompt_truncated,
        stdout_summary: None,
        stderr_summary: Some(message.to_string()),
        stdout_truncated: false,
        stderr_truncated: false,
    }
}

struct OutputCapture<'a> {
    exit_code: Option<i32>,
    timed_out: bool,
    effective_model: Option<&'a str>,
    key_summary: Option<String>,
    prompt: &'a str,
    stdout: &'a [u8],
    stderr: &'a [u8],
}

fn event_from_output(
    profile: &Profile,
    started_at: chrono::DateTime<Local>,
    ended_at: chrono::DateTime<Local>,
    capture: OutputCapture<'_>,
) -> ProbeEvent {
    let (stdout_summary, stdout_truncated) =
        summarize_and_redact(capture.stdout, profile.stdout_summary_limit_bytes);
    let (stderr_summary, stderr_truncated) =
        summarize_and_redact(capture.stderr, profile.stderr_summary_limit_bytes);
    let stdout_for_classification = String::from_utf8_lossy(capture.stdout);
    let stderr_for_classification = String::from_utf8_lossy(capture.stderr);
    let (prompt_summary, prompt_truncated) = prompt_summary(capture.prompt);
    let classification = classify(
        capture.exit_code,
        capture.timed_out,
        &stdout_for_classification,
        &stderr_for_classification,
    );

    ProbeEvent {
        id: Uuid::new_v4().to_string(),
        profile_id: profile.id.clone(),
        started_at,
        ended_at,
        duration_ms: (ended_at - started_at).num_milliseconds(),
        status: classification.status,
        error_kind: classification.error_kind,
        exit_code: capture.exit_code,
        base_url: profile.base_url.clone(),
        model: event_model(capture.effective_model),
        key_summary: capture.key_summary,
        prompt_summary,
        prompt_truncated,
        stdout_summary,
        stderr_summary,
        stdout_truncated,
        stderr_truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;
    use tempfile::tempdir;

    fn test_profile() -> Profile {
        Profile {
            token: Some("sk-test-token".to_string()),
            ..Profile::default()
        }
    }

    #[tokio::test]
    async fn missing_token_still_uses_claude_code_existing_config() {
        let profile = Profile::default();

        let event = run_probe_with_binary(&profile, "definitely-missing-claude-test-binary").await;

        assert_eq!(event.status, ProbeStatus::ConfigError);
        assert_eq!(event.error_kind.as_deref(), Some("claude_not_found"));
    }

    #[tokio::test]
    async fn empty_base_url_does_not_block_probe() {
        let profile = Profile {
            base_url: " ".to_string(),
            token: None,
            ..Profile::default()
        };

        let event = run_probe_with_binary(&profile, "definitely-missing-claude-test-binary").await;

        assert_eq!(event.status, ProbeStatus::ConfigError);
        assert_eq!(event.error_kind.as_deref(), Some("claude_not_found"));
    }

    #[tokio::test]
    async fn empty_model_does_not_block_probe() {
        let profile = Profile {
            model: " ".to_string(),
            token: None,
            ..Profile::default()
        };

        let event = run_probe_with_binary(&profile, "definitely-missing-claude-test-binary").await;

        assert_eq!(event.status, ProbeStatus::ConfigError);
        assert_eq!(event.error_kind.as_deref(), Some("claude_not_found"));
    }

    #[test]
    fn effective_model_appends_1m_to_supported_models() {
        let profile = Profile {
            model: "sonnet".to_string(),
            ..Profile::default()
        };

        assert_eq!(
            effective_model_with_default(&profile, None).as_deref(),
            Some("sonnet[1m]")
        );
    }

    #[test]
    fn effective_model_uses_detected_default_when_profile_model_is_empty() {
        let profile = Profile::default();

        assert_eq!(
            effective_model_with_default(&profile, Some("sonnet")).as_deref(),
            Some("sonnet[1m]")
        );
        assert_eq!(event_model(Some("sonnet[1m]")), "sonnet[1m]");
    }

    #[test]
    fn effective_model_does_not_override_claude_code_default_when_unresolved() {
        let profile = Profile::default();

        assert_eq!(effective_model_with_default(&profile, None), None);
        assert_eq!(event_model(None), "default");
    }

    #[test]
    fn effective_model_keeps_existing_context_suffix() {
        let profile = Profile {
            model: "claude-opus-4-8[1m]".to_string(),
            ..Profile::default()
        };

        assert_eq!(
            effective_model_with_default(&profile, None).as_deref(),
            Some("claude-opus-4-8[1m]")
        );
    }

    #[test]
    fn effective_model_appends_1m_to_custom_models() {
        let profile = Profile {
            model: "gateway-model-name".to_string(),
            ..Profile::default()
        };

        assert_eq!(
            effective_model_with_default(&profile, None).as_deref(),
            Some("gateway-model-name[1m]")
        );
    }

    #[test]
    fn effective_model_does_not_add_1m_to_known_non_1m_models() {
        let profile = Profile {
            model: "haiku".to_string(),
            ..Profile::default()
        };

        assert_eq!(
            effective_model_with_default(&profile, None).as_deref(),
            Some("haiku")
        );
    }

    #[test]
    fn effective_model_respects_native_context() {
        let profile = Profile {
            model: "claude-sonnet-4-6".to_string(),
            context_size: "native".to_string(),
            ..Profile::default()
        };

        assert_eq!(
            effective_model_with_default(&profile, None).as_deref(),
            Some("claude-sonnet-4-6")
        );
    }

    #[test]
    fn effort_is_passed_only_for_supported_levels() {
        assert_eq!(normalized_effort("low"), Some("low"));
        assert_eq!(normalized_effort("xhigh"), Some("xhigh"));
        assert_eq!(normalized_effort("ultracode"), None);
        assert_eq!(normalized_effort(""), None);
    }

    #[test]
    fn prompt_pool_candidates_filter_and_trim_values() {
        let candidates = prompt_pool_candidates(&[
            "  hi  ".to_string(),
            String::new(),
            "ping".to_string(),
            "x".repeat(MAX_PROMPT_CHARS + 5),
        ]);

        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0], "hi");
        assert_eq!(candidates[1], "ping");
        assert_eq!(candidates[2].chars().count(), MAX_PROMPT_CHARS);
    }

    #[test]
    fn select_probe_prompt_uses_pool_before_fallback() {
        let profile = Profile {
            prompt: "fallback".to_string(),
            prompt_pool: vec!["pool prompt".to_string()],
            ..Profile::default()
        };

        assert_eq!(select_probe_prompt(&profile), "pool prompt");
        assert_eq!(fallback_prompt(" "), FALLBACK_PROMPT);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn spawned_probe_passes_effective_model_and_effort_to_cli() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("temp dir");
        let capture_path = dir.path().join("claude-args.txt");
        let script_path = dir.path().join("fake-claude");
        std::fs::write(
            &script_path,
            r#"#!/bin/sh
printf '%s\n' "$@" > "$ANYROUTER_KEEPER_CAPTURE_ARGS"
printf '{"result":"OK"}'
"#,
        )
        .expect("write fake claude");
        let mut permissions = std::fs::metadata(&script_path)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script_path, permissions).expect("chmod fake claude");
        std::env::set_var("ANYROUTER_KEEPER_CAPTURE_ARGS", &capture_path);

        let profile = Profile {
            token: None,
            base_url: String::new(),
            model: "gateway-model".to_string(),
            effort: "max".to_string(),
            context_size: "1m".to_string(),
            ..Profile::default()
        };

        let event =
            run_probe_with_binary(&profile, script_path.to_str().expect("script path")).await;
        let args = std::fs::read_to_string(&capture_path).expect("read captured args");

        assert_eq!(event.status, ProbeStatus::Success);
        assert_eq!(event.model, "gateway-model[1m]");
        assert!(args.contains("--model\ngateway-model[1m]"));
        assert!(args.contains("--effort\nmax"));
        assert!(args.contains("--safe-mode"));
        assert!(args.contains("--disable-slash-commands"));
        assert!(args.contains("--strict-mcp-config"));
        assert!(args.contains("--mcp-config\n{\"mcpServers\":{}}"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn spawned_probe_writes_prompt_pool_item_to_stdin() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("temp dir");
        let capture_path = dir.path().join("claude-stdin.txt");
        let script_path = dir.path().join("fake-claude-stdin");
        std::fs::write(
            &script_path,
            r#"#!/bin/sh
cat > "$ANYROUTER_KEEPER_CAPTURE_STDIN"
printf '{"result":"OK"}'
"#,
        )
        .expect("write fake claude");
        let mut permissions = std::fs::metadata(&script_path)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script_path, permissions).expect("chmod fake claude");
        std::env::set_var("ANYROUTER_KEEPER_CAPTURE_STDIN", &capture_path);

        let profile = Profile {
            token: None,
            base_url: String::new(),
            prompt: "fallback".to_string(),
            prompt_pool: vec!["pool prompt".to_string()],
            ..Profile::default()
        };

        let event =
            run_probe_with_binary(&profile, script_path.to_str().expect("script path")).await;
        let stdin = std::fs::read_to_string(&capture_path).expect("read captured stdin");

        assert_eq!(event.status, ProbeStatus::Success);
        assert_eq!(stdin, "pool prompt");
        assert_eq!(event.prompt_summary.as_deref(), Some("pool prompt"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn spawned_probe_waits_for_stream_retry_process() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("temp dir");
        let script_path = dir.path().join("fake-claude-retry");
        std::fs::write(
            &script_path,
            r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"api_retry","error_status":429,"error":"rate_limit"}'
sleep 1
printf '%s\n' '{"type":"result","subtype":"error","is_error":true,"result":"API Error: Request rejected (429)"}'
exit 1
"#,
        )
        .expect("write fake claude");
        let mut permissions = std::fs::metadata(&script_path)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script_path, permissions).expect("chmod fake claude");

        let profile = Profile {
            token: None,
            base_url: String::new(),
            timeout_seconds: 20,
            ..Profile::default()
        };

        let started = std::time::Instant::now();
        let event =
            run_probe_with_binary(&profile, script_path.to_str().expect("script path")).await;

        assert!(started.elapsed() >= StdDuration::from_secs(1));
        assert_eq!(event.status, ProbeStatus::QueueMiss);
        assert_eq!(event.error_kind.as_deref(), Some("429"));
        assert!(event
            .stdout_summary
            .as_deref()
            .is_some_and(|stdout| stdout.contains("api_retry")));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn spawned_probe_classifies_retry_beyond_summary_limit() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("temp dir");
        let script_path = dir.path().join("fake-claude-long-init");
        std::fs::write(
            &script_path,
            r#"#!/bin/sh
printf '{"type":"system","subtype":"init","padding":"'
python3 - <<'PY'
print('x' * 4096, end='')
PY
printf '"}\n'
printf '%s\n' '{"type":"system","subtype":"api_retry","error_status":429,"error":"rate_limit"}'
exit 1
"#,
        )
        .expect("write fake claude");
        let mut permissions = std::fs::metadata(&script_path)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script_path, permissions).expect("chmod fake claude");

        let profile = Profile {
            token: None,
            base_url: String::new(),
            stdout_summary_limit_bytes: 256,
            ..Profile::default()
        };

        let event =
            run_probe_with_binary(&profile, script_path.to_str().expect("script path")).await;

        assert_eq!(event.status, ProbeStatus::QueueMiss);
        assert_eq!(event.error_kind.as_deref(), Some("429"));
        assert!(event.stdout_truncated);
    }

    #[tokio::test]
    async fn invalid_token_kind_is_config_error_without_spawning_cli() {
        let profile = Profile {
            token_kind: "BAD_ENV".to_string(),
            ..test_profile()
        };

        let event = run_probe_with_binary(&profile, "definitely-missing-claude-test-binary").await;

        assert_eq!(event.status, ProbeStatus::ConfigError);
        assert_eq!(event.error_kind.as_deref(), Some("invalid_token_kind"));
    }

    #[tokio::test]
    async fn invalid_token_kind_without_token_does_not_block_existing_claude_config() {
        let profile = Profile {
            token: None,
            token_kind: "BAD_ENV".to_string(),
            ..Profile::default()
        };

        let event = run_probe_with_binary(&profile, "definitely-missing-claude-test-binary").await;

        assert_eq!(event.status, ProbeStatus::ConfigError);
        assert_eq!(event.error_kind.as_deref(), Some("claude_not_found"));
    }

    #[tokio::test]
    async fn configured_claude_path_is_used_by_probe() {
        let profile = Profile {
            claude_binary_path: "definitely-missing-configured-claude".to_string(),
            token: None,
            ..Profile::default()
        };

        let event = run_probe(&profile).await;

        assert_eq!(event.status, ProbeStatus::ConfigError);
        assert_eq!(event.error_kind.as_deref(), Some("claude_not_found"));
    }

    #[tokio::test]
    async fn live_claude_probe_uses_existing_local_config_when_enabled() {
        if std::env::var("ANYROUTER_KEEPER_RUN_LIVE_TESTS")
            .ok()
            .as_deref()
            != Some("1")
        {
            return;
        }

        let profile = Profile {
            token: None,
            base_url: String::new(),
            model: String::new(),
            timeout_seconds: 8,
            ..Profile::default()
        };

        let event = run_probe(&profile).await;

        assert!(
            matches!(
                event.status,
                ProbeStatus::Success | ProbeStatus::QueueMiss | ProbeStatus::Timeout
            ),
            "live probe should use local Claude Code config and not fail before probing: {event:?}",
        );

        let status = event.status.clone();
        let dir = tempdir().expect("temp dir");
        let db = Database::open(dir.path().join("live-probe.sqlite3")).expect("open db");
        db.migrate().expect("migrate db");
        db.push_event(event).expect("push live event");
        db.flush_buffer().expect("flush live event");

        let events = db.list_events(10, None).expect("list live events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, status);
    }
}
