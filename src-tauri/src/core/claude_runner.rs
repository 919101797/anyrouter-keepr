use std::process::Stdio;
use std::time::Duration as StdDuration;

use chrono::Local;
use rand::seq::SliceRandom;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

use crate::core::classifier::classify;
use crate::core::claude_installation::{apply_claude_command_path, resolve_claude_binary};
use crate::core::claude_runtime_config::detect_claude_runtime_config;
use crate::core::redactor::summarize_and_redact;
use crate::core::types::{ProbeEvent, ProbeStatus, Profile};

const FALLBACK_PROMPT: &str = "只回复 OK";
const MAX_PROMPT_CHARS: usize = 1_000;
const MAX_PROMPT_SUMMARY_BYTES: usize = 1_024;

pub async fn run_probe(profile: &Profile) -> ProbeEvent {
    match resolve_claude_binary(&profile.claude_binary_path).await {
        Ok(resolution) => run_probe_with_binary(profile, &resolution.effective_path).await,
        Err(error) => claude_not_found_event(profile, &error.message),
    }
}

pub fn claude_not_found_event(profile: &Profile, message: &str) -> ProbeEvent {
    let started_at = Local::now();
    let effective_model = effective_model(profile);
    let probe_prompt = select_probe_prompt(profile);
    config_error_event(
        profile,
        started_at,
        effective_model.as_deref(),
        &probe_prompt,
        "claude_not_found",
        message,
    )
}

async fn run_probe_with_binary(profile: &Profile, binary: &str) -> ProbeEvent {
    let started_at = Local::now();
    let token = profile.token.clone().unwrap_or_default();
    let effective_model = effective_model(profile);
    let probe_prompt = select_probe_prompt(profile);

    if !token.trim().is_empty() && !is_supported_token_kind(&profile.token_kind) {
        return config_error_event(
            profile,
            started_at,
            effective_model.as_deref(),
            &probe_prompt,
            "invalid_token_kind",
            "token kind must be ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY",
        );
    }

    let mut child =
        match build_command(profile, binary, token.trim(), effective_model.as_deref()).spawn() {
            Ok(child) => child,
            Err(err) => {
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
    let output_result = timeout(timeout_duration, child.wait_with_output()).await;
    let ended_at = Local::now();

    match output_result {
        Ok(Ok(output)) => event_from_output(
            profile,
            started_at,
            ended_at,
            OutputCapture {
                exit_code: output.status.code(),
                timed_out: false,
                effective_model: effective_model.as_deref(),
                prompt: &probe_prompt,
                stdout: &output.stdout,
                stderr: &output.stderr,
            },
        ),
        Ok(Err(err)) => {
            let stderr = err.to_string();
            event_from_output(
                profile,
                started_at,
                ended_at,
                OutputCapture {
                    exit_code: None,
                    timed_out: false,
                    effective_model: effective_model.as_deref(),
                    prompt: &probe_prompt,
                    stdout: b"",
                    stderr: stderr.as_bytes(),
                },
            )
        }
        Err(_) => {
            let stderr = format!("probe timed out after {}s", profile.timeout_seconds);
            event_from_output(
                profile,
                started_at,
                ended_at,
                OutputCapture {
                    exit_code: None,
                    timed_out: true,
                    effective_model: effective_model.as_deref(),
                    prompt: &probe_prompt,
                    stdout: b"",
                    stderr: stderr.as_bytes(),
                },
            )
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
) -> Command {
    let mut command = Command::new(binary);
    apply_claude_command_path(&mut command, binary);
    command
        .arg("-p")
        .arg("--no-session-persistence")
        .arg("--tools")
        .arg("")
        .arg("--output-format")
        .arg("json")
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

    command
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
    let (prompt_summary, prompt_truncated) = prompt_summary(capture.prompt);
    let classification = classify(
        capture.exit_code,
        capture.timed_out,
        stdout_summary.as_deref().unwrap_or_default(),
        stderr_summary.as_deref().unwrap_or_default(),
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
