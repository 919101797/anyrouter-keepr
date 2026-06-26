use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde_json::Value;

use crate::core::types::ClaudeRuntimeConfig;

pub fn detect_claude_runtime_config() -> ClaudeRuntimeConfig {
    let checked_at = Local::now().to_rfc3339();
    let mut errors = Vec::new();

    let model = detect_model(&mut errors);
    let effort = detect_effort(&mut errors);

    ClaudeRuntimeConfig {
        checked_at,
        default_model: model.as_ref().map(|candidate| candidate.value.clone()),
        model_source: model
            .map(|candidate| candidate.source)
            .unwrap_or_else(|| "claude_code_default_unresolved".to_string()),
        default_effort: effort.as_ref().map(|candidate| candidate.value.clone()),
        effort_source: effort
            .map(|candidate| candidate.source)
            .unwrap_or_else(|| "not_configured".to_string()),
        error: if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        },
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    value: String,
    source: String,
}

fn detect_model(errors: &mut Vec<String>) -> Option<Candidate> {
    detect_env_value(model_env_keys(), "environment")
        .or_else(|| detect_claude_settings(model_keys(), errors))
        .or_else(|| detect_cc_switch_value(model_keys(), "model", errors))
        .or_else(|| detect_cc_switch_recent_request_model(errors))
        .or_else(|| detect_cc_switch_stream_check_model(errors))
        .or_else(|| detect_claude_json_model(errors))
}

fn detect_effort(errors: &mut Vec<String>) -> Option<Candidate> {
    detect_env_value(effort_env_keys(), "environment")
        .or_else(|| detect_claude_settings(effort_keys(), errors))
        .or_else(|| detect_cc_switch_value(effort_keys(), "effort", errors))
        .and_then(normalize_effort_candidate)
}

fn model_env_keys() -> &'static [&'static str] {
    &[
        "ANTHROPIC_MODEL",
        "ANTHROPIC_DEFAULT_MODEL",
        "CLAUDE_MODEL",
        "CLAUDE_CODE_MODEL",
    ]
}

fn effort_env_keys() -> &'static [&'static str] {
    &["CLAUDE_CODE_EFFORT", "CLAUDE_EFFORT", "ANTHROPIC_EFFORT"]
}

fn model_keys() -> &'static [&'static str] {
    &[
        "model",
        "defaultmodel",
        "primarymodel",
        "anthropic_model",
        "anthropicdefaultmodel",
        "anthropic_default_model",
        "claudemodel",
        "claude_model",
        "claudecodemodel",
        "claude_code_model",
    ]
}

fn effort_keys() -> &'static [&'static str] {
    &[
        "effort",
        "effortlevel",
        "effort_level",
        "claudeeffort",
        "claude_effort",
        "claudecodeeffort",
        "claude_code_effort",
    ]
}

fn detect_env_value(keys: &[&str], source_prefix: &str) -> Option<Candidate> {
    for key in keys {
        if let Ok(value) = env::var(key) {
            if let Some(value) = clean_value(&value) {
                return Some(Candidate {
                    value,
                    source: format!("{source_prefix}:{key}"),
                });
            }
        }
    }
    None
}

fn detect_claude_settings(keys: &[&str], errors: &mut Vec<String>) -> Option<Candidate> {
    for path in claude_settings_paths() {
        let Some(value) = read_json(&path, errors) else {
            continue;
        };
        if let Some(found) = find_value_by_keys(&value, keys) {
            return Some(Candidate {
                value: found,
                source: format!("claude_settings:{}", display_path(&path)),
            });
        }
    }
    None
}

fn claude_settings_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".claude/settings.local.json"));
        paths.push(home.join(".claude/settings.json"));
    }
    if let Ok(cwd) = env::current_dir() {
        paths.push(cwd.join(".claude/settings.local.json"));
        paths.push(cwd.join(".claude/settings.json"));
    }
    paths
}

fn detect_cc_switch_value(
    keys: &[&str],
    value_kind: &str,
    errors: &mut Vec<String>,
) -> Option<Candidate> {
    let current_provider = current_cc_switch_provider_id(errors);
    let conn = open_cc_switch_db(errors)?;

    let row = query_current_cc_switch_provider(&conn, current_provider.as_deref(), errors)?;
    let settings_config = match serde_json::from_str::<Value>(&row.settings_config) {
        Ok(value) => value,
        Err(err) => {
            errors.push(format!("cc-switch provider config: {err}"));
            return None;
        }
    };

    find_value_by_keys(&settings_config, keys).map(|value| Candidate {
        value,
        source: format!("cc_switch:{value_kind}:{}", row.name),
    })
}

fn current_cc_switch_provider_id(errors: &mut Vec<String>) -> Option<String> {
    let home = dirs::home_dir()?;
    let settings_path = home.join(".cc-switch/settings.json");
    read_json(&settings_path, errors)
        .and_then(|value| {
            value
                .get("currentProviderClaude")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|value| !value.trim().is_empty())
}

fn open_cc_switch_db(errors: &mut Vec<String>) -> Option<Connection> {
    let home = dirs::home_dir()?;
    let db_path = home.join(".cc-switch/cc-switch.db");
    if !db_path.exists() {
        return None;
    }

    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    match Connection::open_with_flags(&db_path, flags) {
        Ok(conn) => Some(conn),
        Err(err) => {
            errors.push(format!("cc-switch db: {err}"));
            None
        }
    }
}

struct ProviderRow {
    name: String,
    settings_config: String,
}

fn query_current_cc_switch_provider(
    conn: &Connection,
    current_provider: Option<&str>,
    errors: &mut Vec<String>,
) -> Option<ProviderRow> {
    if let Some(provider_id) = current_provider {
        match conn
            .query_row(
                "SELECT name, settings_config FROM providers WHERE app_type='claude' AND id=?1 LIMIT 1",
                [provider_id],
                |row| {
                    Ok(ProviderRow {
                        name: row.get(0)?,
                        settings_config: row.get(1)?,
                    })
                },
            )
            .optional()
        {
            Ok(Some(row)) => return Some(row),
            Ok(None) => {}
            Err(err) => errors.push(format!("cc-switch current provider: {err}")),
        }
    }

    match conn
        .query_row(
            "SELECT name, settings_config FROM providers WHERE app_type='claude' AND is_current=1 LIMIT 1",
            [],
            |row| {
                Ok(ProviderRow {
                    name: row.get(0)?,
                    settings_config: row.get(1)?,
                })
            },
        )
        .optional()
    {
        Ok(row) => row,
        Err(err) => {
            errors.push(format!("cc-switch current provider: {err}"));
            None
        }
    }
}

fn detect_cc_switch_recent_request_model(errors: &mut Vec<String>) -> Option<Candidate> {
    let current_provider = current_cc_switch_provider_id(errors);
    let conn = open_cc_switch_db(errors)?;
    let model = query_recent_request_model(&conn, current_provider.as_deref(), errors)?;

    clean_value(&model).map(|value| Candidate {
        value,
        source: "cc_switch:recent_request_model".to_string(),
    })
}

fn query_recent_request_model(
    conn: &Connection,
    current_provider: Option<&str>,
    errors: &mut Vec<String>,
) -> Option<String> {
    if let Some(provider_id) = current_provider {
        match conn
            .query_row(
                r#"
                SELECT COALESCE(NULLIF(TRIM(request_model), ''), NULLIF(TRIM(model), ''))
                FROM proxy_request_logs
                WHERE app_type = 'claude'
                  AND provider_id = ?1
                  AND COALESCE(NULLIF(TRIM(request_model), ''), NULLIF(TRIM(model), '')) IS NOT NULL
                ORDER BY created_at DESC
                LIMIT 1
                "#,
                params![provider_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
        {
            Ok(Some(model)) => return Some(model),
            Ok(None) => {}
            Err(err) => errors.push(format!("cc-switch recent model: {err}")),
        }
    }

    match conn
        .query_row(
            r#"
            SELECT COALESCE(NULLIF(TRIM(request_model), ''), NULLIF(TRIM(model), ''))
            FROM proxy_request_logs
            WHERE app_type = 'claude'
              AND COALESCE(NULLIF(TRIM(request_model), ''), NULLIF(TRIM(model), '')) IS NOT NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
    {
        Ok(model) => model,
        Err(err) => {
            errors.push(format!("cc-switch recent model: {err}"));
            None
        }
    }
}

fn detect_cc_switch_stream_check_model(errors: &mut Vec<String>) -> Option<Candidate> {
    let conn = open_cc_switch_db(errors)?;
    let config = match conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'stream_check_config' LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
    {
        Ok(config) => config,
        Err(err) => {
            errors.push(format!("cc-switch stream check config: {err}"));
            None
        }
    }?;
    let value = match serde_json::from_str::<Value>(&config) {
        Ok(value) => value,
        Err(err) => {
            errors.push(format!("cc-switch stream check config: {err}"));
            return None;
        }
    };

    value
        .get("claudeModel")
        .and_then(Value::as_str)
        .and_then(clean_value)
        .map(|value| Candidate {
            value,
            source: "cc_switch:stream_check_config:claudeModel".to_string(),
        })
}

fn detect_claude_json_model(errors: &mut Vec<String>) -> Option<Candidate> {
    let home = dirs::home_dir()?;
    let path = home.join(".claude.json");
    let value = read_json(&path, errors)?;
    if value
        .get("hasOpusPlanDefault")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Some(Candidate {
            value: "opusplan".to_string(),
            source: "claude_state:hasOpusPlanDefault".to_string(),
        });
    }
    None
}

fn find_value_by_keys(value: &Value, keys: &[&str]) -> Option<String> {
    let key_set = keys.iter().copied().collect::<HashSet<_>>();
    find_value_by_keys_inner(value, &key_set)
}

fn find_value_by_keys_inner(value: &Value, keys: &HashSet<&str>) -> Option<String> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let normalized = normalize_key(key);
                if keys.contains(normalized.as_str()) {
                    if let Some(value) = value_to_clean_string(child) {
                        return Some(value);
                    }
                }
            }
            for child in map.values() {
                if let Some(value) = find_value_by_keys_inner(child, keys) {
                    return Some(value);
                }
            }
            None
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_value_by_keys_inner(item, keys)),
        _ => None,
    }
}

fn value_to_clean_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => clean_value(value),
        Value::Number(value) => clean_value(&value.to_string()),
        _ => None,
    }
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn normalize_effort_candidate(candidate: Candidate) -> Option<Candidate> {
    let normalized = candidate.value.trim().to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "low" | "medium" | "high" | "xhigh" | "max"
    ) {
        Some(Candidate {
            value: normalized,
            source: candidate.source,
        })
    } else {
        None
    }
}

fn clean_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn read_json(path: &Path, errors: &mut Vec<String>) -> Option<Value> {
    if !path.exists() {
        return None;
    }
    match fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(value) => Some(value),
            Err(err) => {
                errors.push(format!("{}: {err}", display_path(path)));
                None
            }
        },
        Err(err) => {
            errors.push(format!("{}: {err}", display_path(path)));
            None
        }
    }
}

fn display_path(path: &Path) -> String {
    let Some(home) = dirs::home_dir() else {
        return path.display().to_string();
    };
    path.strip_prefix(&home)
        .map(|path| format!("~/{}", path.display()))
        .unwrap_or_else(|_| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_nested_model_values() {
        let value = serde_json::json!({
            "env": {
                "ANTHROPIC_MODEL": "opus"
            }
        });

        assert_eq!(
            find_value_by_keys(&value, model_keys()).as_deref(),
            Some("opus")
        );
    }

    #[test]
    fn normalizes_effort_values() {
        let candidate = Candidate {
            value: "HIGH".to_string(),
            source: "test".to_string(),
        };

        let normalized = normalize_effort_candidate(candidate).unwrap();

        assert_eq!(normalized.value, "high");
    }

    #[test]
    fn recent_request_model_falls_back_to_session_rows() {
        let conn = Connection::open_in_memory().expect("open sqlite");
        conn.execute_batch(
            r#"
            CREATE TABLE proxy_request_logs (
              provider_id TEXT NOT NULL,
              app_type TEXT NOT NULL,
              model TEXT NOT NULL,
              request_model TEXT,
              created_at INTEGER NOT NULL
            );
            INSERT INTO proxy_request_logs
              (provider_id, app_type, model, request_model, created_at)
            VALUES
              ('_session', 'claude', 'claude-opus-4-8', 'claude-opus-4-8', 20),
              ('other-provider', 'claude', 'claude-haiku-4-5', 'claude-haiku-4-5', 10);
            "#,
        )
        .expect("seed proxy request logs");

        let mut errors = Vec::new();
        let model = query_recent_request_model(&conn, Some("current-provider"), &mut errors);

        assert_eq!(model.as_deref(), Some("claude-opus-4-8"));
        assert!(errors.is_empty());
    }
}
