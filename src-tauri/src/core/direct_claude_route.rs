use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use reqwest::Url;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::core::cc_switch_paths;

pub const DEFAULT_UPSTREAM_URL: &str = "https://anyrouter.top";
pub const KEEPER_UPSTREAM_ENV_KEY: &str = "ANYROUTER_KEEPER_UPSTREAM_URL";

const BASE_URL_ENV_KEY: &str = "ANTHROPIC_BASE_URL";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectClaudeRouteStatus {
    pub enabled: bool,
    pub proxy_url: String,
    pub upstream_url: String,
    pub upstream_source: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamResolution {
    pub upstream_url: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsConfigRewrite {
    pub settings_config: String,
    pub changed: bool,
    pub upstream_url: Option<String>,
}

pub fn keeper_proxy_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

pub fn should_follow_cc_switch(profile_base_url: &str, port: u16) -> bool {
    let trimmed = profile_base_url.trim();
    trimmed.is_empty() || is_keeper_proxy_url(trimmed, port)
}

pub fn is_keeper_proxy_url(value: &str, port: u16) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }

    if let Ok(url) = Url::parse(value.trim_end_matches('/')) {
        let host_matches = matches!(
            url.host_str(),
            Some("127.0.0.1") | Some("localhost") | Some("::1") | Some("[::1]")
        );
        let port_matches = url.port_or_known_default() == Some(port);
        let path_matches = url.path().is_empty() || url.path() == "/";
        return url.scheme() == "http" && host_matches && port_matches && path_matches;
    }

    let normalized = value.trim_end_matches('/');
    normalized == format!("127.0.0.1:{port}") || normalized == format!("localhost:{port}")
}

pub fn resolve_proxy_upstream_url(
    profile_base_url: &str,
    configured_upstream_url: &str,
    follow_cc_switch: bool,
    port: u16,
) -> UpstreamResolution {
    if let Some(profile_url) = clean_non_keeper_url(profile_base_url, port) {
        return UpstreamResolution {
            upstream_url: profile_url,
            source: "profile".to_string(),
        };
    }

    if follow_cc_switch {
        if let Some(resolution) = detect_current_cc_switch_upstream_url(port) {
            return resolution;
        }
        if let Some(resolution) = detect_claude_settings_upstream_url(port) {
            return resolution;
        }
    }

    if let Some(configured_url) = clean_non_keeper_url(configured_upstream_url, port) {
        return UpstreamResolution {
            upstream_url: configured_url,
            source: "proxy_config".to_string(),
        };
    }

    if !follow_cc_switch {
        if let Some(resolution) = detect_current_cc_switch_upstream_url(port) {
            return resolution;
        }
        if let Some(resolution) = detect_claude_settings_upstream_url(port) {
            return resolution;
        }
    }

    UpstreamResolution {
        upstream_url: DEFAULT_UPSTREAM_URL.to_string(),
        source: "default".to_string(),
    }
}

pub fn sync_direct_claude_route(port: u16) -> DirectClaudeRouteStatus {
    let proxy_url = keeper_proxy_url(port);
    let mut errors = Vec::new();

    if let Err(err) = rewrite_claude_settings_file(&proxy_url, port) {
        errors.push(err);
    }
    if let Err(err) = rewrite_cc_switch_providers(&proxy_url, port) {
        errors.push(err);
    }

    let upstream = detect_current_cc_switch_upstream_url(port)
        .or_else(|| detect_claude_settings_upstream_url(port))
        .unwrap_or_else(|| UpstreamResolution {
            upstream_url: DEFAULT_UPSTREAM_URL.to_string(),
            source: "default".to_string(),
        });

    DirectClaudeRouteStatus {
        enabled: route_enabled_in_claude_settings(&proxy_url)
            || route_enabled_in_any_cc_switch_provider(&proxy_url),
        proxy_url,
        upstream_url: upstream.upstream_url,
        upstream_source: upstream.source,
        error: if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        },
    }
}

pub fn disable_direct_claude_route(port: u16) -> DirectClaudeRouteStatus {
    let proxy_url = keeper_proxy_url(port);
    let mut errors = Vec::new();

    if let Err(err) = restore_claude_settings_file(&proxy_url) {
        errors.push(err);
    }
    if let Err(err) = restore_cc_switch_providers(&proxy_url) {
        errors.push(err);
    }

    let upstream = detect_current_cc_switch_upstream_url(port)
        .or_else(|| detect_claude_settings_upstream_url(port))
        .unwrap_or_else(|| UpstreamResolution {
            upstream_url: DEFAULT_UPSTREAM_URL.to_string(),
            source: "default".to_string(),
        });

    DirectClaudeRouteStatus {
        enabled: route_enabled_in_claude_settings(&proxy_url)
            || route_enabled_in_any_cc_switch_provider(&proxy_url),
        proxy_url,
        upstream_url: upstream.upstream_url,
        upstream_source: upstream.source,
        error: if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        },
    }
}

pub fn rewrite_settings_config_for_keeper_proxy(
    settings_config: &str,
    proxy_url: &str,
    fallback_upstream_url: Option<&str>,
) -> Result<SettingsConfigRewrite, String> {
    let mut value = serde_json::from_str::<Value>(settings_config)
        .map_err(|err| format!("provider settings_config json: {err}"))?;
    let fallback =
        fallback_upstream_url.and_then(|value| clean_non_keeper_url(value, proxy_port(proxy_url)));
    let rewrite =
        rewrite_settings_value_for_keeper_proxy(&mut value, proxy_url, fallback.as_deref());

    Ok(SettingsConfigRewrite {
        settings_config: serde_json::to_string(&value)
            .map_err(|err| format!("provider settings_config serialize: {err}"))?,
        changed: rewrite.changed,
        upstream_url: rewrite.upstream_url,
    })
}

pub fn restore_settings_config_from_keeper_proxy(
    settings_config: &str,
    proxy_url: &str,
) -> Result<SettingsConfigRewrite, String> {
    let mut value = serde_json::from_str::<Value>(settings_config)
        .map_err(|err| format!("provider settings_config json: {err}"))?;
    let rewrite = restore_settings_value_from_keeper_proxy(&mut value, proxy_url);

    Ok(SettingsConfigRewrite {
        settings_config: serde_json::to_string(&value)
            .map_err(|err| format!("provider settings_config serialize: {err}"))?,
        changed: rewrite.changed,
        upstream_url: rewrite.upstream_url,
    })
}

fn rewrite_settings_value_for_keeper_proxy(
    value: &mut Value,
    proxy_url: &str,
    fallback_upstream_url: Option<&str>,
) -> SettingsConfigRewrite {
    let port = proxy_port(proxy_url);
    let env = ensure_env_object(value);
    let current_base_url = env_string(env, BASE_URL_ENV_KEY);
    let preserved_upstream = env_string(env, KEEPER_UPSTREAM_ENV_KEY);
    let upstream_url = current_base_url
        .as_deref()
        .and_then(|url| clean_non_keeper_url(url, port))
        .or_else(|| {
            preserved_upstream
                .as_deref()
                .and_then(|url| clean_non_keeper_url(url, port))
        })
        .or_else(|| fallback_upstream_url.and_then(|url| clean_non_keeper_url(url, port)));

    let mut changed = false;
    if env_string(env, BASE_URL_ENV_KEY).as_deref() != Some(proxy_url) {
        env.insert(
            BASE_URL_ENV_KEY.to_string(),
            Value::String(proxy_url.to_string()),
        );
        changed = true;
    }
    if let Some(upstream_url) = upstream_url.as_deref() {
        if env_string(env, KEEPER_UPSTREAM_ENV_KEY).as_deref() != Some(upstream_url) {
            env.insert(
                KEEPER_UPSTREAM_ENV_KEY.to_string(),
                Value::String(upstream_url.to_string()),
            );
            changed = true;
        }
    }

    SettingsConfigRewrite {
        settings_config: String::new(),
        changed,
        upstream_url,
    }
}

fn restore_settings_value_from_keeper_proxy(
    value: &mut Value,
    proxy_url: &str,
) -> SettingsConfigRewrite {
    let Some(root) = value.as_object_mut() else {
        return SettingsConfigRewrite {
            settings_config: String::new(),
            changed: false,
            upstream_url: None,
        };
    };
    let Some(env) = root.get_mut("env").and_then(Value::as_object_mut) else {
        return SettingsConfigRewrite {
            settings_config: String::new(),
            changed: false,
            upstream_url: None,
        };
    };

    let current_base_url = env_string(env, BASE_URL_ENV_KEY);
    let upstream_url = env_string(env, KEEPER_UPSTREAM_ENV_KEY)
        .as_deref()
        .and_then(|url| clean_non_keeper_url(url, proxy_port(proxy_url)));
    let mut changed = false;

    if current_base_url.as_deref() == Some(proxy_url) {
        match upstream_url.as_deref() {
            Some(upstream_url) => {
                env.insert(
                    BASE_URL_ENV_KEY.to_string(),
                    Value::String(upstream_url.to_string()),
                );
            }
            None => {
                env.remove(BASE_URL_ENV_KEY);
            }
        }
        changed = true;
    }

    if env.remove(KEEPER_UPSTREAM_ENV_KEY).is_some() {
        changed = true;
    }

    SettingsConfigRewrite {
        settings_config: String::new(),
        changed,
        upstream_url,
    }
}

fn rewrite_claude_settings_file(proxy_url: &str, port: u16) -> Result<(), String> {
    let Some(path) = claude_settings_path() else {
        return Ok(());
    };
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path)
        .map_err(|err| format!("claude settings read {}: {err}", display_path(&path)))?;
    let mut value = serde_json::from_str::<Value>(&content)
        .map_err(|err| format!("claude settings json {}: {err}", display_path(&path)))?;
    let fallback =
        detect_current_cc_switch_upstream_url(port).map(|resolution| resolution.upstream_url);
    let rewrite =
        rewrite_settings_value_for_keeper_proxy(&mut value, proxy_url, fallback.as_deref());
    if rewrite.changed {
        write_backup_once(&path)?;
        fs::write(
            &path,
            serde_json::to_string_pretty(&value).map_err(|err| {
                format!("claude settings serialize {}: {err}", display_path(&path))
            })?,
        )
        .map_err(|err| format!("claude settings write {}: {err}", display_path(&path)))?;
    }
    Ok(())
}

fn restore_claude_settings_file(proxy_url: &str) -> Result<(), String> {
    let Some(path) = claude_settings_path() else {
        return Ok(());
    };
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path)
        .map_err(|err| format!("claude settings read {}: {err}", display_path(&path)))?;
    let mut value = serde_json::from_str::<Value>(&content)
        .map_err(|err| format!("claude settings json {}: {err}", display_path(&path)))?;
    let rewrite = restore_settings_value_from_keeper_proxy(&mut value, proxy_url);
    if rewrite.changed {
        write_backup_once(&path)?;
        fs::write(
            &path,
            serde_json::to_string_pretty(&value).map_err(|err| {
                format!("claude settings serialize {}: {err}", display_path(&path))
            })?,
        )
        .map_err(|err| format!("claude settings write {}: {err}", display_path(&path)))?;
    }
    Ok(())
}

fn rewrite_cc_switch_providers(proxy_url: &str, port: u16) -> Result<(), String> {
    let Some(db_path) = cc_switch_paths::db_path() else {
        return Ok(());
    };
    if !db_path.exists() {
        return Ok(());
    }

    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("cc-switch db open {}: {err}", display_path(&db_path)))?;

    let rows = {
        let mut stmt = conn
            .prepare("SELECT id, settings_config FROM providers WHERE app_type='claude'")
            .map_err(|err| format!("cc-switch providers query: {err}"))?;
        let mapped = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|err| format!("cc-switch providers query: {err}"))?;
        let rows = mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("cc-switch providers row: {err}"))?;
        rows
    };

    for (provider_id, settings_config) in rows {
        let fallback = query_provider_endpoint(&conn, &provider_id, port)?;
        let rewrite = rewrite_settings_config_for_keeper_proxy(
            &settings_config,
            proxy_url,
            fallback.as_deref(),
        )?;
        if rewrite.changed {
            write_backup_once(&db_path)?;
            conn.execute(
                "UPDATE providers SET settings_config=?1 WHERE app_type='claude' AND id=?2",
                params![rewrite.settings_config, provider_id],
            )
            .map_err(|err| format!("cc-switch provider update: {err}"))?;
        }
    }

    Ok(())
}

fn restore_cc_switch_providers(proxy_url: &str) -> Result<(), String> {
    let Some(db_path) = cc_switch_paths::db_path() else {
        return Ok(());
    };
    if !db_path.exists() {
        return Ok(());
    }

    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("cc-switch db open {}: {err}", display_path(&db_path)))?;

    let rows = {
        let mut stmt = conn
            .prepare("SELECT id, settings_config FROM providers WHERE app_type='claude'")
            .map_err(|err| format!("cc-switch providers query: {err}"))?;
        let mapped = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|err| format!("cc-switch providers query: {err}"))?;
        mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("cc-switch providers row: {err}"))?
    };

    for (provider_id, settings_config) in rows {
        let rewrite = restore_settings_config_from_keeper_proxy(&settings_config, proxy_url)?;
        if rewrite.changed {
            write_backup_once(&db_path)?;
            conn.execute(
                "UPDATE providers SET settings_config=?1 WHERE app_type='claude' AND id=?2",
                params![rewrite.settings_config, provider_id],
            )
            .map_err(|err| format!("cc-switch provider update: {err}"))?;
        }
    }

    Ok(())
}

fn detect_current_cc_switch_upstream_url(port: u16) -> Option<UpstreamResolution> {
    let db_path = cc_switch_paths::db_path()?;
    if !db_path.exists() {
        return None;
    }
    let conn = Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    let current_provider = current_cc_switch_provider_id();
    let row = query_current_provider(&conn, current_provider.as_deref()).ok()??;
    let value = serde_json::from_str::<Value>(&row.settings_config).ok()?;
    if let Some(upstream_url) = extract_settings_value_upstream_url(&value, port) {
        return Some(UpstreamResolution {
            upstream_url,
            source: format!("cc_switch:{}", row.name),
        });
    }
    query_provider_endpoint(&conn, &row.id, port)
        .ok()
        .flatten()
        .map(|upstream_url| UpstreamResolution {
            upstream_url,
            source: format!("cc_switch_endpoint:{}", row.name),
        })
}

fn detect_claude_settings_upstream_url(port: u16) -> Option<UpstreamResolution> {
    let mut seen = HashSet::new();
    for path in claude_settings_paths() {
        if !seen.insert(path.clone()) || !path.exists() {
            continue;
        }
        let value = fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str::<Value>(&content).ok());
        if let Some(upstream_url) = value
            .as_ref()
            .and_then(|value| extract_settings_value_upstream_url(value, port))
        {
            return Some(UpstreamResolution {
                upstream_url,
                source: format!("claude_settings:{}", display_path(&path)),
            });
        }
    }
    None
}

fn route_enabled_in_claude_settings(proxy_url: &str) -> bool {
    claude_settings_paths().into_iter().any(|path| {
        fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str::<Value>(&content).ok())
            .and_then(|value| {
                value
                    .get("env")
                    .and_then(Value::as_object)
                    .and_then(|env| env_string(env, BASE_URL_ENV_KEY))
            })
            .as_deref()
            == Some(proxy_url)
    })
}

fn route_enabled_in_any_cc_switch_provider(proxy_url: &str) -> bool {
    let Some(db_path) = cc_switch_paths::db_path() else {
        return false;
    };
    if !db_path.exists() {
        return false;
    }
    let Ok(conn) = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return false;
    };
    let Ok(mut stmt) =
        conn.prepare("SELECT settings_config FROM providers WHERE app_type='claude'")
    else {
        return false;
    };
    let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) else {
        return false;
    };

    let enabled = rows.filter_map(Result::ok).any(|settings_config| {
        serde_json::from_str::<Value>(&settings_config)
            .ok()
            .and_then(|value| {
                value
                    .get("env")
                    .and_then(Value::as_object)
                    .and_then(|env| env_string(env, BASE_URL_ENV_KEY))
            })
            .as_deref()
            == Some(proxy_url)
    });
    enabled
}

fn extract_settings_value_upstream_url(value: &Value, port: u16) -> Option<String> {
    let env = value.get("env").and_then(Value::as_object)?;
    env_string(env, KEEPER_UPSTREAM_ENV_KEY)
        .as_deref()
        .and_then(|url| clean_non_keeper_url(url, port))
        .or_else(|| {
            env_string(env, BASE_URL_ENV_KEY)
                .as_deref()
                .and_then(|url| clean_non_keeper_url(url, port))
        })
}

struct ProviderRow {
    id: String,
    name: String,
    settings_config: String,
}

fn query_current_provider(
    conn: &Connection,
    current_provider: Option<&str>,
) -> Result<Option<ProviderRow>, rusqlite::Error> {
    if let Some(provider_id) = current_provider {
        if let Some(row) = conn
            .query_row(
                "SELECT id, name, settings_config FROM providers WHERE app_type='claude' AND id=?1 LIMIT 1",
                [provider_id],
                |row| {
                    Ok(ProviderRow {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        settings_config: row.get(2)?,
                    })
                },
            )
            .optional()?
        {
            return Ok(Some(row));
        }
    }

    conn.query_row(
        "SELECT id, name, settings_config FROM providers WHERE app_type='claude' AND is_current=1 LIMIT 1",
        [],
        |row| {
            Ok(ProviderRow {
                id: row.get(0)?,
                name: row.get(1)?,
                settings_config: row.get(2)?,
            })
        },
    )
    .optional()
}

fn query_provider_endpoint(
    conn: &Connection,
    provider_id: &str,
    port: u16,
) -> Result<Option<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT url FROM provider_endpoints WHERE app_type='claude' AND provider_id=?1 ORDER BY COALESCE(added_at, 0) DESC, id DESC",
        )
        .map_err(|err| format!("cc-switch provider endpoints query: {err}"))?;
    let rows = stmt
        .query_map([provider_id], |row| row.get::<_, String>(0))
        .map_err(|err| format!("cc-switch provider endpoints query: {err}"))?;
    for row in rows {
        if let Some(url) = clean_non_keeper_url(
            &row.map_err(|err| format!("cc-switch provider endpoint row: {err}"))?,
            port,
        ) {
            return Ok(Some(url));
        }
    }
    Ok(None)
}

fn current_cc_switch_provider_id() -> Option<String> {
    let path = cc_switch_paths::settings_path()?;
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|value| {
            value
                .get("currentProviderClaude")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
}

fn claude_settings_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".claude/settings.json"))
}

fn claude_settings_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".claude/settings.local.json"));
        paths.push(home.join(".claude/settings.json"));
    }
    paths
}

fn ensure_env_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    let root = value.as_object_mut().expect("root object just ensured");
    if !root.get("env").is_some_and(Value::is_object) {
        root.insert("env".to_string(), Value::Object(Map::new()));
    }
    root.get_mut("env")
        .and_then(Value::as_object_mut)
        .expect("env object just ensured")
}

fn env_string(env: &Map<String, Value>, key: &str) -> Option<String> {
    env.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn clean_non_keeper_url(value: &str, port: u16) -> Option<String> {
    let value = clean_url(value)?;
    if is_keeper_proxy_url(&value, port) {
        None
    } else {
        Some(value)
    }
}

fn clean_url(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches('/');
    if value.is_empty() {
        return None;
    }
    if value.starts_with("http://") || value.starts_with("https://") {
        return Some(value.to_string());
    }
    Some(format!("https://{value}"))
}

fn proxy_port(proxy_url: &str) -> u16 {
    Url::parse(proxy_url)
        .ok()
        .and_then(|url| url.port_or_known_default())
        .unwrap_or(15800)
}

fn display_path(path: &Path) -> String {
    let Some(home) = dirs::home_dir() else {
        return path.display().to_string();
    };
    path.strip_prefix(&home)
        .map(|path| format!("~/{}", path.display()))
        .unwrap_or_else(|_| path.display().to_string())
}

fn write_backup_once(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return Ok(());
    };
    let backup_path = path.with_file_name(format!("{file_name}.anyrouter-keeper.bak"));
    if backup_path.exists() {
        return Ok(());
    }
    fs::copy(path, &backup_path).map_err(|err| {
        format!(
            "backup {} -> {}: {err}",
            display_path(path),
            display_path(&backup_path)
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn rewrites_provider_base_url_to_keeper_and_preserves_real_upstream() {
        let input = r#"{"env":{"ANTHROPIC_BASE_URL":"https://anyrouter.top","ANTHROPIC_AUTH_TOKEN":"secret"}}"#;

        let rewrite =
            rewrite_settings_config_for_keeper_proxy(input, "http://127.0.0.1:15800", None)
                .expect("provider config should rewrite");
        let parsed: Value = serde_json::from_str(&rewrite.settings_config).unwrap();
        let env = parsed.get("env").and_then(Value::as_object).unwrap();

        assert!(rewrite.changed);
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("http://127.0.0.1:15800")
        );
        assert_eq!(
            env.get(KEEPER_UPSTREAM_ENV_KEY).and_then(Value::as_str),
            Some("https://anyrouter.top")
        );
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN").and_then(Value::as_str),
            Some("secret")
        );
    }

    #[test]
    fn keeps_existing_preserved_upstream_when_provider_already_points_at_keeper() {
        let input = r#"{"env":{"ANTHROPIC_BASE_URL":"http://127.0.0.1:15800","ANYROUTER_KEEPER_UPSTREAM_URL":"https://anyrouter.top"}}"#;

        let rewrite = rewrite_settings_config_for_keeper_proxy(
            input,
            "http://127.0.0.1:15800",
            Some("https://fallback.example"),
        )
        .expect("provider config should parse");
        let parsed: Value = serde_json::from_str(&rewrite.settings_config).unwrap();
        let env = parsed.get("env").and_then(Value::as_object).unwrap();

        assert!(!rewrite.changed);
        assert_eq!(
            env.get(KEEPER_UPSTREAM_ENV_KEY).and_then(Value::as_str),
            Some("https://anyrouter.top")
        );
    }

    #[test]
    fn restores_provider_base_url_from_keeper_to_preserved_upstream() {
        let input = r#"{"env":{"ANTHROPIC_BASE_URL":"http://127.0.0.1:15800","ANYROUTER_KEEPER_UPSTREAM_URL":"https://anyrouter.top","ANTHROPIC_AUTH_TOKEN":"secret"}}"#;

        let rewrite = restore_settings_config_from_keeper_proxy(input, "http://127.0.0.1:15800")
            .expect("provider config should restore");
        let parsed: Value = serde_json::from_str(&rewrite.settings_config).unwrap();
        let env = parsed.get("env").and_then(Value::as_object).unwrap();

        assert!(rewrite.changed);
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("https://anyrouter.top")
        );
        assert_eq!(env.get(KEEPER_UPSTREAM_ENV_KEY), None);
        assert_eq!(
            env.get("ANTHROPIC_AUTH_TOKEN").and_then(Value::as_str),
            Some("secret")
        );
    }

    #[test]
    fn restore_leaves_non_keeper_provider_base_url_unchanged() {
        let input = r#"{"env":{"ANTHROPIC_BASE_URL":"https://other.example","ANYROUTER_KEEPER_UPSTREAM_URL":"https://anyrouter.top"}}"#;

        let rewrite = restore_settings_config_from_keeper_proxy(input, "http://127.0.0.1:15800")
            .expect("provider config should parse");
        let parsed: Value = serde_json::from_str(&rewrite.settings_config).unwrap();
        let env = parsed.get("env").and_then(Value::as_object).unwrap();

        assert!(rewrite.changed);
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str),
            Some("https://other.example")
        );
        assert_eq!(env.get(KEEPER_UPSTREAM_ENV_KEY), None);
    }

    #[test]
    fn detects_keeper_loopback_urls_for_the_proxy_port() {
        assert!(is_keeper_proxy_url("http://127.0.0.1:15800", 15800));
        assert!(is_keeper_proxy_url("http://localhost:15800/", 15800));
        assert!(!is_keeper_proxy_url("https://anyrouter.top", 15800));
        assert!(!is_keeper_proxy_url("http://127.0.0.1:15900", 15800));
    }
}
