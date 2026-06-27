use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{DateTime, Duration, Local, Timelike};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;
use uuid::Uuid;

use crate::core::types::{
    default_prompt_pool, ActivityBucket, ClaudeDetectionLog, ClaudeInstallation, ProbeEvent,
    ProbeEventDto, ProbeStatus, Profile, StoredProfile,
};
use crate::security::keychain;
use crate::storage::event_buffer::EventBuffer;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("keychain error: {0}")]
    Keychain(#[from] keychain::KeychainError),
    #[error("time parse error: {0}")]
    TimeParse(#[from] chrono::ParseError),
}

pub struct Database {
    conn: Mutex<Connection>,
    buffer: Mutex<EventBuffer>,
    path: PathBuf,
}

impl Database {
    pub fn open_default() -> Result<Self, DbError> {
        let base_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join("anyrouter-claude-keeper");
        fs::create_dir_all(&base_dir)?;
        Self::open(base_dir.join("keeper.sqlite3"))
    }

    pub fn open(path: PathBuf) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "busy_timeout", 3000)?;

        Ok(Self {
            conn: Mutex::new(conn),
            buffer: Mutex::new(EventBuffer::new(5, 300)),
            path,
        })
    }

    pub fn migrate(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS profiles (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              claude_binary_path TEXT NOT NULL DEFAULT '',
              base_url TEXT NOT NULL,
              token_kind TEXT NOT NULL,
              model TEXT NOT NULL,
              effort TEXT NOT NULL DEFAULT 'low',
              context_size TEXT NOT NULL DEFAULT '1m',
              prompt TEXT NOT NULL,
              prompt_pool TEXT NOT NULL DEFAULT '[]',
              min_interval_seconds INTEGER NOT NULL,
              max_interval_seconds INTEGER NOT NULL,
              timeout_seconds INTEGER NOT NULL,
              start_time TEXT NOT NULL,
              end_time TEXT NOT NULL,
              enabled INTEGER NOT NULL,
              stdout_summary_limit_bytes INTEGER NOT NULL,
              stderr_summary_limit_bytes INTEGER NOT NULL,
              event_flush_count INTEGER NOT NULL,
              event_flush_interval_seconds INTEGER NOT NULL,
              history_retention_days INTEGER NOT NULL,
              max_events_per_profile INTEGER NOT NULL,
              max_database_size_mb INTEGER NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS probe_events (
              id TEXT PRIMARY KEY,
              profile_id TEXT NOT NULL,
              started_at TEXT NOT NULL,
              ended_at TEXT NOT NULL,
              duration_ms INTEGER NOT NULL,
              status TEXT NOT NULL,
              error_kind TEXT,
              exit_code INTEGER,
              base_url TEXT NOT NULL,
              model TEXT NOT NULL,
              key_summary TEXT,
              prompt_summary TEXT,
              prompt_truncated INTEGER NOT NULL DEFAULT 0,
              stdout_summary TEXT,
              stderr_summary TEXT,
              stdout_truncated INTEGER NOT NULL,
              stderr_truncated INTEGER NOT NULL,
              created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_probe_events_profile_started
              ON probe_events(profile_id, started_at DESC);

            CREATE TABLE IF NOT EXISTS claude_detection_logs (
              id TEXT PRIMARY KEY,
              checked_at TEXT NOT NULL,
              configured_path TEXT NOT NULL,
              detected_path TEXT,
              effective_path TEXT,
              version TEXT,
              source TEXT NOT NULL,
              status TEXT NOT NULL,
              error TEXT,
              created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_claude_detection_logs_checked
              ON claude_detection_logs(checked_at DESC);

            CREATE TABLE IF NOT EXISTS app_state (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            "#,
        )?;
        add_column_if_missing(
            &conn,
            "profiles",
            "claude_binary_path",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        add_column_if_missing(&conn, "profiles", "effort", "TEXT NOT NULL DEFAULT 'low'")?;
        add_column_if_missing(
            &conn,
            "profiles",
            "context_size",
            "TEXT NOT NULL DEFAULT '1m'",
        )?;
        let prompt_pool_added = add_column_if_missing(
            &conn,
            "profiles",
            "prompt_pool",
            "TEXT NOT NULL DEFAULT '[]'",
        )?;
        if prompt_pool_added {
            conn.execute(
                "UPDATE profiles SET prompt_pool = ?1 WHERE prompt_pool = '[]'",
                params![prompt_pool_json(&default_prompt_pool())],
            )?;
        }
        add_column_if_missing(&conn, "probe_events", "prompt_summary", "TEXT")?;
        add_column_if_missing(&conn, "probe_events", "key_summary", "TEXT")?;
        add_column_if_missing(
            &conn,
            "probe_events",
            "prompt_truncated",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        Ok(())
    }

    pub fn get_profile(&self) -> Result<StoredProfile, DbError> {
        let profile = self.get_profile_raw()?.unwrap_or_default();
        let has_token = keychain::get_token(&profile.id).ok().flatten().is_some();
        let mut stored = StoredProfile::from(profile);
        stored.has_token = has_token;
        Ok(stored)
    }

    pub fn get_runtime_profile(&self) -> Result<Profile, DbError> {
        let mut profile = self.get_profile_raw()?.unwrap_or_default();
        profile.token = keychain::get_token(&profile.id).ok().flatten();
        Ok(profile)
    }

    fn get_profile_raw(&self) -> Result<Option<Profile>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let profile = conn
            .query_row(
                r#"
                SELECT id, name, base_url, token_kind, model, effort, context_size, prompt, prompt_pool,
                       claude_binary_path, min_interval_seconds, max_interval_seconds, timeout_seconds,
                       start_time, end_time, enabled,
                       stdout_summary_limit_bytes, stderr_summary_limit_bytes,
                       event_flush_count, event_flush_interval_seconds,
                       history_retention_days, max_events_per_profile, max_database_size_mb
                FROM profiles WHERE id = 'default'
                "#,
                [],
                |row| {
                    Ok(Profile {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        base_url: row.get(2)?,
                        token: None,
                        token_kind: row.get(3)?,
                        model: row.get(4)?,
                        effort: row.get(5)?,
                        context_size: row.get(6)?,
                        prompt: row.get(7)?,
                        prompt_pool: parse_prompt_pool(&row.get::<_, String>(8)?),
                        claude_binary_path: row.get(9)?,
                        min_interval_seconds: row.get::<_, i64>(10)? as u64,
                        max_interval_seconds: row.get::<_, i64>(11)? as u64,
                        timeout_seconds: row.get::<_, i64>(12)? as u64,
                        start_time: row.get(13)?,
                        end_time: row.get(14)?,
                        enabled: row.get::<_, i64>(15)? != 0,
                        stdout_summary_limit_bytes: row.get::<_, i64>(16)? as usize,
                        stderr_summary_limit_bytes: row.get::<_, i64>(17)? as usize,
                        event_flush_count: row.get::<_, i64>(18)? as usize,
                        event_flush_interval_seconds: row.get::<_, i64>(19)? as u64,
                        history_retention_days: row.get(20)?,
                        max_events_per_profile: row.get(21)?,
                        max_database_size_mb: row.get(22)?,
                    })
                },
            )
            .optional()?;
        Ok(profile)
    }

    pub fn save_profile(&self, mut profile: Profile) -> Result<StoredProfile, DbError> {
        profile.id = "default".to_string();
        if let Some(token) = profile
            .token
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            keychain::set_token(&profile.id, token)?;
        }
        profile.prompt_pool = normalize_prompt_pool(profile.prompt_pool);
        let prompt_pool_json = prompt_pool_json(&profile.prompt_pool);
        let now = Local::now().to_rfc3339();
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            r#"
            INSERT INTO profiles (
                id, name, claude_binary_path, base_url, token_kind, model, effort, context_size, prompt, prompt_pool,
                min_interval_seconds, max_interval_seconds, timeout_seconds,
                start_time, end_time, enabled,
                stdout_summary_limit_bytes, stderr_summary_limit_bytes,
                event_flush_count, event_flush_interval_seconds,
                history_retention_days, max_events_per_profile, max_database_size_mb,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?24)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                claude_binary_path = excluded.claude_binary_path,
                base_url = excluded.base_url,
                token_kind = excluded.token_kind,
                model = excluded.model,
                effort = excluded.effort,
                context_size = excluded.context_size,
                prompt = excluded.prompt,
                prompt_pool = excluded.prompt_pool,
                min_interval_seconds = excluded.min_interval_seconds,
                max_interval_seconds = excluded.max_interval_seconds,
                timeout_seconds = excluded.timeout_seconds,
                start_time = excluded.start_time,
                end_time = excluded.end_time,
                enabled = excluded.enabled,
                stdout_summary_limit_bytes = excluded.stdout_summary_limit_bytes,
                stderr_summary_limit_bytes = excluded.stderr_summary_limit_bytes,
                event_flush_count = excluded.event_flush_count,
                event_flush_interval_seconds = excluded.event_flush_interval_seconds,
                history_retention_days = excluded.history_retention_days,
                max_events_per_profile = excluded.max_events_per_profile,
                max_database_size_mb = excluded.max_database_size_mb,
                updated_at = excluded.updated_at
            "#,
            params![
                profile.id,
                profile.name,
                profile.claude_binary_path,
                profile.base_url,
                profile.token_kind,
                profile.model,
                profile.effort,
                profile.context_size,
                profile.prompt,
                prompt_pool_json,
                profile.min_interval_seconds as i64,
                profile.max_interval_seconds as i64,
                profile.timeout_seconds as i64,
                profile.start_time,
                profile.end_time,
                if profile.enabled { 1 } else { 0 },
                profile.stdout_summary_limit_bytes as i64,
                profile.stderr_summary_limit_bytes as i64,
                profile.event_flush_count as i64,
                profile.event_flush_interval_seconds as i64,
                profile.history_retention_days,
                profile.max_events_per_profile,
                profile.max_database_size_mb,
                now
            ],
        )?;
        drop(conn);
        self.reset_buffer(
            profile.event_flush_count,
            profile.event_flush_interval_seconds,
        );
        self.get_profile()
    }

    pub fn record_claude_detection(
        &self,
        installation: &ClaudeInstallation,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let now = Local::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO claude_detection_logs (
              id, checked_at, configured_path, detected_path, effective_path,
              version, source, status, error, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                Uuid::new_v4().to_string(),
                &installation.checked_at,
                &installation.configured_path,
                installation.detected_path.as_deref(),
                installation.effective_path.as_deref(),
                installation.version.as_deref(),
                &installation.source,
                &installation.status,
                installation.error.as_deref(),
                now,
            ],
        )?;
        conn.execute(
            r#"
            DELETE FROM claude_detection_logs
            WHERE id IN (
              SELECT id FROM claude_detection_logs
              ORDER BY checked_at DESC
              LIMIT -1 OFFSET 200
            )
            "#,
            [],
        )?;
        Ok(())
    }

    pub fn list_claude_detection_logs(
        &self,
        limit: i64,
    ) -> Result<Vec<ClaudeDetectionLog>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, checked_at, configured_path, detected_path, effective_path,
                   version, source, status, error
            FROM claude_detection_logs
            ORDER BY checked_at DESC
            LIMIT ?1
            "#,
        )?;
        let rows = stmt.query_map(params![limit.clamp(1, 200)], |row| {
            Ok(ClaudeDetectionLog {
                id: row.get(0)?,
                checked_at: row.get(1)?,
                configured_path: row.get(2)?,
                detected_path: row.get(3)?,
                effective_path: row.get(4)?,
                version: row.get(5)?,
                source: row.get(6)?,
                status: row.get(7)?,
                error: row.get(8)?,
            })
        })?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(row?);
        }
        Ok(logs)
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<(), DbError> {
        let mut profile = self.get_runtime_profile()?;
        profile.enabled = enabled;
        self.save_profile(profile)?;
        Ok(())
    }

    pub fn push_event(&self, event: ProbeEvent) -> Result<(), DbError> {
        let should_flush = {
            let mut buffer = self.buffer.lock().expect("event buffer mutex poisoned");
            buffer.push(event)
        };
        if should_flush {
            self.flush_buffer()?;
        }
        Ok(())
    }

    pub fn flush_buffer(&self) -> Result<(), DbError> {
        let events = {
            let mut buffer = self.buffer.lock().expect("event buffer mutex poisoned");
            if buffer.is_empty() {
                return Ok(());
            }
            buffer.drain()
        };

        let mut conn = self.conn.lock().expect("db mutex poisoned");
        let tx = conn.transaction()?;
        for event in events {
            tx.execute(
                r#"
                INSERT INTO probe_events (
                    id, profile_id, started_at, ended_at, duration_ms, status,
                    error_kind, exit_code, base_url, model, key_summary, prompt_summary,
                    prompt_truncated, stdout_summary, stderr_summary,
                    stdout_truncated, stderr_truncated, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                "#,
                params![
                    event.id,
                    event.profile_id,
                    event.started_at.to_rfc3339(),
                    event.ended_at.to_rfc3339(),
                    event.duration_ms,
                    event.status.as_str(),
                    event.error_kind,
                    event.exit_code,
                    event.base_url,
                    event.model,
                    event.key_summary,
                    event.prompt_summary,
                    if event.prompt_truncated { 1 } else { 0 },
                    event.stdout_summary,
                    event.stderr_summary,
                    if event.stdout_truncated { 1 } else { 0 },
                    if event.stderr_truncated { 1 } else { 0 },
                    Local::now().to_rfc3339(),
                ],
            )?;
        }
        tx.commit()?;
        drop(conn);

        let profile = self.get_profile_raw()?.unwrap_or_default();
        self.apply_retention_limits(&profile)?;
        if self.database_size_bytes()? > profile.max_database_size_mb.max(1) * 1024 * 1024 {
            self.enforce_database_size_limit(profile.max_database_size_mb)?;
        }
        Ok(())
    }

    pub fn list_events(
        &self,
        limit: i64,
        status: Option<String>,
    ) -> Result<Vec<ProbeEventDto>, DbError> {
        self.flush_buffer()?;
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut events = Vec::new();
        if let Some(status) = status {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, profile_id, started_at, ended_at, duration_ms, status, error_kind,
                       exit_code, base_url, model, key_summary, prompt_summary, prompt_truncated,
                       stdout_summary, stderr_summary, stdout_truncated, stderr_truncated
                FROM probe_events
                WHERE profile_id = 'default' AND status = ?1
                ORDER BY started_at DESC
                LIMIT ?2
                "#,
            )?;
            let rows = stmt.query_map(params![status, limit], map_event_row)?;
            for row in rows {
                events.push(row?);
            }
        } else {
            let mut stmt = conn.prepare(
                r#"
                SELECT id, profile_id, started_at, ended_at, duration_ms, status, error_kind,
                       exit_code, base_url, model, key_summary, prompt_summary, prompt_truncated,
                       stdout_summary, stderr_summary, stdout_truncated, stderr_truncated
                FROM probe_events
                WHERE profile_id = 'default'
                ORDER BY started_at DESC
                LIMIT ?1
                "#,
            )?;
            let rows = stmt.query_map(params![limit], map_event_row)?;
            for row in rows {
                events.push(row?);
            }
        }
        Ok(events)
    }

    pub fn last_event(&self) -> Result<Option<ProbeEventDto>, DbError> {
        Ok(self.list_events(1, None)?.into_iter().next())
    }

    pub fn last_success_at(&self) -> Result<Option<String>, DbError> {
        self.flush_buffer()?;
        let conn = self.conn.lock().expect("db mutex poisoned");
        let value = conn
            .query_row(
                "SELECT started_at FROM probe_events WHERE profile_id='default' AND status='success' ORDER BY started_at DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value)
    }

    pub fn consecutive_queue_miss(&self) -> Result<u64, DbError> {
        self.flush_buffer()?;
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT status FROM probe_events WHERE profile_id='default' ORDER BY started_at DESC LIMIT 2000",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut count = 0;
        for row in rows {
            match row?.as_str() {
                "queue_miss" | "timeout" => count += 1,
                _ => break,
            }
        }
        Ok(count)
    }

    pub fn activity_summary(&self, hours: i64) -> Result<Vec<ActivityBucket>, DbError> {
        self.flush_buffer()?;
        let since = Local::now() - Duration::hours(hours);
        let events = self.events_since(since)?;
        let mut buckets = std::collections::BTreeMap::<String, ActivityBucket>::new();

        for event in events {
            let parsed = DateTime::parse_from_rfc3339(&event.started_at)?.with_timezone(&Local);
            let minute = parsed.minute();
            let bucket_minute = minute - (minute % 5);
            let bucket_time = parsed
                .with_second(0)
                .unwrap()
                .with_nanosecond(0)
                .unwrap()
                .with_minute(bucket_minute)
                .unwrap();
            let key = bucket_time.to_rfc3339();
            let bucket = buckets.entry(key.clone()).or_insert(ActivityBucket {
                bucket_start: key,
                success_count: 0,
                queue_miss_count: 0,
                timeout_count: 0,
                config_error_count: 0,
                unknown_count: 0,
                last_status: None,
                last_latency_ms: None,
            });
            match event.status {
                ProbeStatus::Success => bucket.success_count += 1,
                ProbeStatus::QueueMiss => bucket.queue_miss_count += 1,
                ProbeStatus::Timeout => bucket.timeout_count += 1,
                ProbeStatus::ConfigError => bucket.config_error_count += 1,
                ProbeStatus::Unknown => bucket.unknown_count += 1,
            }
            bucket.last_status = Some(event.status);
            bucket.last_latency_ms = Some(event.duration_ms);
        }

        Ok(buckets.into_values().collect())
    }

    fn events_since(&self, since: DateTime<Local>) -> Result<Vec<ProbeEventDto>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            r#"
            SELECT id, profile_id, started_at, ended_at, duration_ms, status, error_kind,
                   exit_code, base_url, model, key_summary, prompt_summary, prompt_truncated,
                   stdout_summary, stderr_summary, stdout_truncated, stderr_truncated
            FROM probe_events
            WHERE profile_id = 'default' AND started_at >= ?1
            ORDER BY started_at ASC
            "#,
        )?;
        let rows = stmt.query_map(params![since.to_rfc3339()], map_event_row)?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    pub fn compact_storage(&self) -> Result<(), DbError> {
        self.flush_buffer()?;
        let profile = self.get_profile_raw()?.unwrap_or_default();
        self.apply_retention_limits(&profile)?;
        self.enforce_database_size_limit(profile.max_database_size_mb)?;
        Ok(())
    }

    fn apply_retention_limits(&self, profile: &Profile) -> Result<(), DbError> {
        let cutoff = Local::now() - Duration::days(profile.history_retention_days.max(1));
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            "DELETE FROM probe_events WHERE profile_id='default' AND started_at < ?1",
            params![cutoff.to_rfc3339()],
        )?;
        conn.execute(
            r#"
            DELETE FROM probe_events
            WHERE id IN (
              SELECT id FROM probe_events
              WHERE profile_id='default'
              ORDER BY started_at DESC
              LIMIT -1 OFFSET ?1
            )
            "#,
            params![profile.max_events_per_profile.max(1)],
        )?;
        Ok(())
    }

    fn enforce_database_size_limit(&self, max_database_size_mb: i64) -> Result<(), DbError> {
        let max_bytes = max_database_size_mb.max(1) * 1024 * 1024;
        if self.database_size_bytes()? <= max_bytes {
            return Ok(());
        }

        loop {
            {
                let conn = self.conn.lock().expect("db mutex poisoned");
                if count_events(&conn)? == 0 {
                    break;
                }

                let deleted = conn.execute(
                    r#"
                    DELETE FROM probe_events
                    WHERE id IN (
                      SELECT id FROM probe_events
                      WHERE profile_id='default'
                      ORDER BY started_at ASC
                      LIMIT 500
                    )
                    "#,
                    [],
                )?;
                if deleted == 0 {
                    break;
                }
            }

            self.reclaim_sqlite_space()?;
            if self.database_size_bytes()? <= max_bytes {
                return Ok(());
            }
        }

        self.reclaim_sqlite_space()?;
        Ok(())
    }

    fn database_size_bytes(&self) -> Result<i64, DbError> {
        let mut total = 0_i64;
        for path in [
            self.path.clone(),
            self.path.with_extension("sqlite3-wal"),
            self.path.with_extension("sqlite3-shm"),
        ] {
            if let Ok(metadata) = fs::metadata(path) {
                total += metadata.len() as i64;
            }
        }
        Ok(total)
    }

    fn reclaim_sqlite_space(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute_batch(
            r#"
            PRAGMA wal_checkpoint(TRUNCATE);
            VACUUM;
            "#,
        )?;
        Ok(())
    }

    fn reset_buffer(&self, flush_count: usize, flush_interval_seconds: u64) {
        let mut buffer = self.buffer.lock().expect("event buffer mutex poisoned");
        *buffer = EventBuffer::new(flush_count, flush_interval_seconds);
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

const MAX_PROMPT_POOL_ITEMS: usize = 50;
const MAX_PROMPT_CHARS: usize = 1_000;

fn parse_prompt_pool(value: &str) -> Vec<String> {
    let parsed = serde_json::from_str::<Vec<String>>(value).unwrap_or_default();
    let normalized = normalize_prompt_pool(parsed);
    if normalized == legacy_prompt_pool() || normalized == short_prompt_pool() {
        default_prompt_pool()
    } else {
        normalized
    }
}

fn normalize_prompt_pool(pool: Vec<String>) -> Vec<String> {
    pool.into_iter()
        .filter_map(|prompt| {
            let trimmed = prompt.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trim_to_chars(trimmed, MAX_PROMPT_CHARS))
            }
        })
        .take(MAX_PROMPT_POOL_ITEMS)
        .collect()
}

fn prompt_pool_json(pool: &[String]) -> String {
    serde_json::to_string(pool).unwrap_or_else(|_| "[]".to_string())
}

fn trim_to_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn legacy_prompt_pool() -> Vec<String> {
    ["只回复 OK", "请只回复 OK", "hi", "ping", "请回复 ready"]
        .into_iter()
        .map(ToString::to_string)
        .collect()
}

fn short_prompt_pool() -> Vec<String> {
    [
        "ok", "hi", "ping", "pong", "ack", "yes", "go", "up", "on", "run", "rdy", "chk", "stat",
        "live", "beat", "tick", "tap", "echo", "noop", "test", "mark", "trace", "node", "edge",
        "route", "gw", "api", "cc", "ar", "keep", "pulse", "warm", "wake", "link", "path", "hold",
        "sync", "green", "ready", "ok?", "ping?", "1", "2", "3",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<bool, DbError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        if row? == column {
            return Ok(false);
        }
    }

    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )?;
    Ok(true)
}

fn count_events(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM probe_events", [], |row| row.get(0))
}

fn map_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProbeEventDto> {
    Ok(ProbeEventDto {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        duration_ms: row.get(4)?,
        status: ProbeStatus::parse_status(&row.get::<_, String>(5)?),
        error_kind: row.get(6)?,
        exit_code: row.get(7)?,
        base_url: row.get(8)?,
        model: row.get(9)?,
        key_summary: row.get(10)?,
        prompt_summary: row.get(11)?,
        prompt_truncated: row.get::<_, i64>(12)? != 0,
        stdout_summary: row.get(13)?,
        stderr_summary: row.get(14)?,
        stdout_truncated: row.get::<_, i64>(15)? != 0,
        stderr_truncated: row.get::<_, i64>(16)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use chrono::Local;
    use tempfile::tempdir;
    use uuid::Uuid;

    use super::*;

    fn test_db() -> Database {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("test.sqlite3")).unwrap();
        db.migrate().unwrap();
        db
    }

    fn probe_event_at(index: i64, stderr: &str) -> ProbeEvent {
        let now = Local::now() + Duration::seconds(index);
        ProbeEvent {
            id: Uuid::new_v4().to_string(),
            profile_id: "default".to_string(),
            started_at: now,
            ended_at: now,
            duration_ms: 42,
            status: ProbeStatus::QueueMiss,
            error_kind: Some("429".to_string()),
            exit_code: Some(1),
            base_url: "https://anyrouter.top".to_string(),
            model: "sonnet".to_string(),
            key_summary: Some("profile_override · ANTHROPIC_AUTH_TOKEN · sk-...st-key".to_string()),
            prompt_summary: Some("用一句话讲个笑话".to_string()),
            prompt_truncated: false,
            stdout_summary: None,
            stderr_summary: Some(stderr.to_string()),
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    #[test]
    fn writes_probe_events_after_flush() {
        let db = test_db();
        db.push_event(probe_event_at(0, "HTTP 429")).unwrap();
        db.flush_buffer().unwrap();
        let events = db.list_events(10, None).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, ProbeStatus::QueueMiss);
        assert_eq!(
            events[0].prompt_summary.as_deref(),
            Some("用一句话讲个笑话")
        );
        assert_eq!(
            events[0].key_summary.as_deref(),
            Some("profile_override · ANTHROPIC_AUTH_TOKEN · sk-...st-key")
        );
    }

    #[test]
    fn saves_claude_binary_path_with_profile() {
        let db = test_db();
        db.save_profile(Profile {
            claude_binary_path: "/usr/local/bin/claude".to_string(),
            token: None,
            ..Profile::default()
        })
        .unwrap();

        let profile = db.get_profile().unwrap();

        assert_eq!(profile.claude_binary_path, "/usr/local/bin/claude");
    }

    #[test]
    fn saves_model_execution_options_with_profile() {
        let db = test_db();
        db.save_profile(Profile {
            token: None,
            model: "claude-opus-4-8".to_string(),
            effort: "xhigh".to_string(),
            context_size: "1m".to_string(),
            ..Profile::default()
        })
        .unwrap();

        let profile = db.get_profile().unwrap();

        assert_eq!(profile.model, "claude-opus-4-8");
        assert_eq!(profile.effort, "xhigh");
        assert_eq!(profile.context_size, "1m");
    }

    #[test]
    fn saves_prompt_pool_with_profile() {
        let db = test_db();
        db.save_profile(Profile {
            token: None,
            prompt: "fallback".to_string(),
            prompt_pool: vec![
                "  hi  ".to_string(),
                "".to_string(),
                "ping".to_string(),
                "x".repeat(MAX_PROMPT_CHARS + 20),
            ],
            ..Profile::default()
        })
        .unwrap();

        let profile = db.get_profile().unwrap();

        assert_eq!(profile.prompt, "fallback");
        assert_eq!(profile.prompt_pool.len(), 3);
        assert_eq!(profile.prompt_pool[0], "hi");
        assert_eq!(profile.prompt_pool[1], "ping");
        assert_eq!(profile.prompt_pool[2].chars().count(), MAX_PROMPT_CHARS);
    }

    #[test]
    fn upgrades_legacy_default_prompt_pool() {
        assert_eq!(
            parse_prompt_pool(r#"["只回复 OK","请只回复 OK","hi","ping","请回复 ready"]"#),
            default_prompt_pool()
        );
    }

    #[test]
    fn upgrades_short_default_prompt_pool() {
        assert_eq!(
            parse_prompt_pool(
                r#"["ok","hi","ping","pong","ack","yes","go","up","on","run","rdy","chk","stat","live","beat","tick","tap","echo","noop","test","mark","trace","node","edge","route","gw","api","cc","ar","keep","pulse","warm","wake","link","path","hold","sync","green","ready","ok?","ping?","1","2","3"]"#,
            ),
            default_prompt_pool()
        );
    }

    #[test]
    fn claude_detection_logs_stay_bounded() {
        let db = test_db();

        for index in 0..205 {
            let checked_at = (Local::now() + Duration::seconds(index)).to_rfc3339();
            db.record_claude_detection(&ClaudeInstallation {
                checked_at,
                configured_path: format!("/tmp/claude-{index}"),
                detected_path: None,
                effective_path: Some(format!("/tmp/claude-{index}")),
                version: None,
                source: "manual".to_string(),
                status: "invalid".to_string(),
                error: Some("missing".to_string()),
            })
            .unwrap();
        }

        let logs = db.list_claude_detection_logs(500).unwrap();

        assert_eq!(logs.len(), 200);
        assert_eq!(logs[0].configured_path, "/tmp/claude-204");
        assert_eq!(logs.last().unwrap().configured_path, "/tmp/claude-5");
    }

    #[test]
    fn compaction_respects_event_count_limit() {
        let db = test_db();
        let mut profile = Profile {
            max_events_per_profile: 3,
            ..Profile::default()
        };
        profile.token = None;
        db.save_profile(profile).unwrap();

        for index in 0..6 {
            db.push_event(probe_event_at(index, "HTTP 429")).unwrap();
        }
        db.compact_storage().unwrap();
        assert_eq!(db.list_events(10, None).unwrap().len(), 3);
    }

    #[test]
    fn repeated_error_pressure_stays_bounded_after_automatic_flush() {
        let db = test_db();
        let mut profile = Profile {
            event_flush_count: 10_000,
            max_events_per_profile: 200,
            ..Profile::default()
        };
        profile.token = None;
        db.save_profile(profile).unwrap();

        let long_error = "HTTP 429 ".repeat(512);
        for index in 0..10_000 {
            db.push_event(probe_event_at(index, &long_error)).unwrap();
        }

        let events = db.list_events(1000, None).unwrap();
        assert_eq!(events.len(), 200);
        assert!(events.iter().all(|event| event
            .stderr_summary
            .as_deref()
            .is_none_or(|summary| summary.len() <= 300)));
    }
}
