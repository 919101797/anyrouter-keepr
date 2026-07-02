use std::sync::{Arc, Mutex as StdMutex};

use chrono::{DateTime, Local};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

use crate::core::claude_installation::resolve_claude_binary;
use crate::core::claude_runner::{build_probe_fingerprint, claude_not_found_event, run_probe};
use crate::core::proxy::ProxyHandle;
use crate::core::time_window::{seconds_until, TimeWindow};
use crate::core::types::{AppStatus, ProbeEventDto, ProbeStatus};
use crate::storage::db::Database;
use crate::system::app_log;
use crate::system::power::PowerGuard;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub running: bool,
    pub next_probe_at: Option<String>,
    pub current_state: String,
    pub in_window: bool,
}

pub struct SchedulerHandle {
    db: Arc<Database>,
    proxy: Arc<tokio::sync::Mutex<ProxyHandle>>,
    task: Option<JoinHandle<()>>,
    stop_tx: Option<oneshot::Sender<()>>,
    runtime_status: Arc<StdMutex<RuntimeStatus>>,
    sleep_prevention: Arc<StdMutex<Option<PowerGuard>>>,
}

impl SchedulerHandle {
    pub fn new(db: Arc<Database>, proxy: Arc<tokio::sync::Mutex<ProxyHandle>>) -> Self {
        Self {
            db,
            proxy,
            task: None,
            stop_tx: None,
            runtime_status: Arc::new(StdMutex::new(RuntimeStatus {
                running: false,
                next_probe_at: None,
                current_state: "paused".to_string(),
                in_window: false,
            })),
            sleep_prevention: Arc::new(StdMutex::new(None)),
        }
    }

    pub async fn start(&mut self) -> Result<(), String> {
        if self.task.as_ref().is_some_and(|task| !task.is_finished()) {
            self.db.set_enabled(true).map_err(|err| err.to_string())?;
            self.refresh_sleep_prevention();
            self.update_status(|status| {
                status.running = true;
                status.current_state = "running".to_string();
            });
            return Ok(());
        }

        let profile = self
            .db
            .get_runtime_profile()
            .map_err(|err| err.to_string())?;
        match resolve_claude_binary(&profile.claude_binary_path).await {
            Ok(resolution) => {
                app_log::info(
                    "scheduler.start",
                    format!(
                        "claude source={} effective_path={}",
                        resolution.source, resolution.effective_path
                    ),
                );
            }
            Err(error) => {
                app_log::error(
                    "scheduler.start.resolve_claude",
                    format!("{}; scheduler will keep retrying", error.message),
                );
                let fingerprint = {
                    let proxy = self.proxy.lock().await;
                    let status = proxy.status();
                    build_probe_fingerprint(Some(&status))
                };
                let mut event = claude_not_found_event(&profile, &error.message);
                event.attach_fingerprint(fingerprint);
                let _ = self.db.push_event(event);
                let _ = self.db.flush_buffer();
            }
        }

        self.db.set_enabled(true).map_err(|err| err.to_string())?;
        let (tx, mut rx) = oneshot::channel();
        let db = self.db.clone();
        let proxy = self.proxy.clone();
        let runtime_status = self.runtime_status.clone();
        let sleep_prevention = self.sleep_prevention.clone();
        self.stop_tx = Some(tx);
        sync_sleep_prevention(&sleep_prevention, profile.prevent_sleep);
        self.update_status(|status| {
            status.running = true;
            status.current_state = "running".to_string();
            status.next_probe_at = Some(Local::now().to_rfc3339());
        });

        self.task = Some(tokio::spawn(async move {
            let exit_reason = loop {
                let profile = match db.get_runtime_profile() {
                    Ok(profile) => profile,
                    Err(error) => {
                        app_log::error("scheduler.loop.get_profile", error.to_string());
                        sleep(Duration::from_secs(30)).await;
                        continue;
                    }
                };

                sync_sleep_prevention(&sleep_prevention, profile.prevent_sleep);

                if !profile.enabled {
                    break "profile_disabled".to_string();
                }

                let window = match TimeWindow::parse(&profile.start_time, &profile.end_time) {
                    Ok(window) => window,
                    Err(error) => {
                        app_log::error("scheduler.loop.time_window", error);
                        let next_probe = Local::now() + chrono::Duration::seconds(30);
                        set_runtime_status(&runtime_status, |status| {
                            status.running = true;
                            status.in_window = false;
                            status.current_state = "config_error".to_string();
                            status.next_probe_at = Some(next_probe.to_rfc3339());
                        });
                        tokio::select! {
                            _ = sleep(Duration::from_secs(30)) => {},
                            _ = &mut rx => {
                                break "stop_signal".to_string();
                            },
                        }
                        continue;
                    }
                };

                let now = Local::now();
                if !window.contains(now) {
                    let next_start = window.next_start_after(now);
                    set_runtime_status(&runtime_status, |status| {
                        status.running = true;
                        status.in_window = false;
                        status.current_state = "sleeping".to_string();
                        status.next_probe_at = Some(next_start.to_rfc3339());
                    });
                    let wait = seconds_until(next_start).min(3600);
                    tokio::select! {
                        _ = sleep(Duration::from_secs(wait.max(1))) => {},
                        _ = &mut rx => {
                            break "stop_signal".to_string();
                        },
                    }
                    continue;
                }

                set_runtime_status(&runtime_status, |status| {
                    status.running = true;
                    status.in_window = true;
                    status.current_state = "probing".to_string();
                    status.next_probe_at = None;
                });

                let (proxy_url, fingerprint) = {
                    let proxy = proxy.lock().await;
                    let status = proxy.status();
                    (proxy.proxy_url(), build_probe_fingerprint(Some(&status)))
                };
                let mut event = tokio::select! {
                    event = run_probe(&profile, proxy_url.as_deref()) => event,
                    _ = &mut rx => {
                        break "stop_signal".to_string();
                    },
                };
                event.attach_fingerprint(fingerprint);
                app_log::info(
                    "scheduler.probe",
                    format!(
                        "status={} error={:?} duration_ms={} model={} stderr={}",
                        event.status.as_str(),
                        event.error_kind,
                        event.duration_ms,
                        event.model,
                        app_log::compact(event.stderr_summary.as_deref())
                    ),
                );
                let event_state = match event.status {
                    ProbeStatus::Success => "connected",
                    ProbeStatus::QueueMiss | ProbeStatus::Timeout => "racing",
                    ProbeStatus::ConfigError => "config_error",
                    ProbeStatus::Unknown => "unknown",
                }
                .to_string();
                let _ = db.push_event(event);

                let min = profile
                    .min_interval_seconds
                    .min(profile.max_interval_seconds)
                    .max(1);
                let max = profile.max_interval_seconds.max(min);
                let wait = rand::thread_rng().gen_range(min..=max);
                let next_probe = Local::now() + chrono::Duration::seconds(wait as i64);
                set_runtime_status(&runtime_status, |status| {
                    status.running = true;
                    status.in_window = true;
                    status.current_state = event_state;
                    status.next_probe_at = Some(next_probe.to_rfc3339());
                });
                tokio::select! {
                    _ = sleep(Duration::from_secs(wait)) => {},
                    _ = &mut rx => {
                        break "stop_signal".to_string();
                    },
                }
            };
            app_log::info("scheduler.loop", format!("exiting reason={exit_reason}"));
            let _ = db.flush_buffer();
            clear_sleep_prevention(&sleep_prevention);
            set_runtime_status(&runtime_status, |status| {
                status.running = false;
                if status.current_state != "config_error" {
                    status.current_state = "paused".to_string();
                }
                status.next_probe_at = None;
            });
        }));

        Ok(())
    }

    pub async fn pause(&mut self) -> Result<(), String> {
        self.db.set_enabled(false).map_err(|err| err.to_string())?;
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        self.clear_sleep_prevention();
        self.update_status(|status| {
            status.running = false;
            status.current_state = "paused".to_string();
            status.next_probe_at = None;
        });
        self.db.flush_buffer().map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn runtime_status(&self) -> RuntimeStatus {
        self.runtime_status
            .lock()
            .expect("runtime status mutex poisoned")
            .clone()
    }

    fn update_status(&self, update: impl FnOnce(&mut RuntimeStatus)) {
        set_runtime_status(&self.runtime_status, update);
    }

    pub fn refresh_sleep_prevention(&self) {
        if !self.runtime_status().running {
            self.clear_sleep_prevention();
            return;
        }

        match self.db.get_runtime_profile() {
            Ok(profile) => sync_sleep_prevention(&self.sleep_prevention, profile.prevent_sleep),
            Err(error) => app_log::error("scheduler.power.get_profile", error.to_string()),
        }
    }

    fn clear_sleep_prevention(&self) {
        clear_sleep_prevention(&self.sleep_prevention);
    }
}

fn set_runtime_status(
    runtime_status: &Arc<StdMutex<RuntimeStatus>>,
    update: impl FnOnce(&mut RuntimeStatus),
) {
    let mut status = runtime_status
        .lock()
        .expect("runtime status mutex poisoned");
    update(&mut status);
}

fn sync_sleep_prevention(
    sleep_prevention: &Arc<StdMutex<Option<PowerGuard>>>,
    prevent_sleep: bool,
) {
    let mut guard = sleep_prevention
        .lock()
        .expect("sleep prevention mutex poisoned");

    if prevent_sleep {
        if guard.is_none() {
            match PowerGuard::acquire() {
                Ok(power_guard) => {
                    app_log::info("scheduler.power", "prevent_sleep enabled");
                    *guard = Some(power_guard);
                }
                Err(error) => app_log::error("scheduler.power", error),
            }
        }
        return;
    }

    if guard.take().is_some() {
        app_log::info("scheduler.power", "prevent_sleep disabled");
    }
}

fn clear_sleep_prevention(sleep_prevention: &Arc<StdMutex<Option<PowerGuard>>>) {
    let mut guard = sleep_prevention
        .lock()
        .expect("sleep prevention mutex poisoned");
    if guard.take().is_some() {
        app_log::info("scheduler.power", "prevent_sleep released");
    }
}

pub fn derive_status(
    profile_id: String,
    runtime: RuntimeStatus,
    last_event: Option<ProbeEventDto>,
    last_success_at: Option<String>,
    consecutive_queue_miss: u64,
) -> AppStatus {
    let current_state = if runtime.current_state == "config_error" {
        "config_error".to_string()
    } else if runtime.current_state == "sleeping" {
        "sleeping".to_string()
    } else if runtime.current_state == "probing" {
        "probing".to_string()
    } else if !runtime.running {
        "paused".to_string()
    } else if let Some(event) = last_event.as_ref() {
        match event.status {
            ProbeStatus::Success => "connected".to_string(),
            ProbeStatus::QueueMiss | ProbeStatus::Timeout => "racing".to_string(),
            ProbeStatus::ConfigError => "config_error".to_string(),
            ProbeStatus::Unknown => "unknown".to_string(),
        }
    } else {
        "running".to_string()
    };

    AppStatus {
        profile_id,
        running: runtime.running,
        current_state,
        last_event,
        last_success_at,
        consecutive_queue_miss,
        next_probe_at: runtime.next_probe_at,
        in_window: runtime.in_window,
    }
}

pub fn next_probe_time(now: DateTime<Local>, min: u64, max: u64) -> DateTime<Local> {
    let min = min.min(max).max(1);
    let max = max.max(min);
    let wait = rand::thread_rng().gen_range(min..=max);
    now + chrono::Duration::seconds(wait as i64)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};

    use super::*;
    use crate::core::proxy::ProxyConfig;
    use crate::core::types::Profile;

    fn live_tests_enabled() -> bool {
        std::env::var("ANYROUTER_KEEPER_RUN_LIVE_TESTS")
            .ok()
            .as_deref()
            == Some("1")
    }

    #[tokio::test]
    async fn start_keeps_running_when_claude_path_is_missing() {
        let dir = tempdir().expect("temp dir");
        let db = Arc::new(Database::open(dir.path().join("scheduler.sqlite3")).expect("open db"));
        db.migrate().expect("migrate db");
        db.save_profile(Profile {
            claude_binary_path: dir
                .path()
                .join("missing-claude")
                .to_string_lossy()
                .into_owned(),
            token: None,
            start_time: "00:00".to_string(),
            end_time: "24:00".to_string(),
            ..Profile::default()
        })
        .expect("save profile");

        let proxy = Arc::new(tokio::sync::Mutex::new(ProxyHandle::new(
            ProxyConfig::default(),
            15800,
        )));
        let mut scheduler = SchedulerHandle::new(db.clone(), proxy);
        let result = scheduler.start().await;

        assert!(result.is_ok());
        let status = scheduler.runtime_status();
        assert!(status.running);

        let events = db.list_events(10, None).expect("list events");
        assert!(!events.is_empty());
        assert!(events.iter().any(|event| {
            event.status == ProbeStatus::ConfigError
                && event.error_kind.as_deref() == Some("claude_not_found")
        }));

        scheduler.pause().await.expect("pause scheduler");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn pause_returns_promptly_while_probe_is_running() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        use tokio::time::timeout;

        let dir = tempdir().expect("temp dir");
        let claude_path = dir.path().join("sleeping-claude");
        fs::write(&claude_path, "#!/bin/sh\nsleep 30\n").expect("write fake claude");
        let mut permissions = fs::metadata(&claude_path)
            .expect("fake claude metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&claude_path, permissions).expect("mark fake claude executable");

        let db = Arc::new(Database::open(dir.path().join("scheduler.sqlite3")).expect("open db"));
        db.migrate().expect("migrate db");
        db.save_profile(Profile {
            claude_binary_path: claude_path.to_string_lossy().into_owned(),
            token: None,
            timeout_seconds: 30,
            start_time: "00:00".to_string(),
            end_time: "24:00".to_string(),
            ..Profile::default()
        })
        .expect("save profile");

        let proxy = Arc::new(tokio::sync::Mutex::new(ProxyHandle::new(
            ProxyConfig::default(),
            15800,
        )));
        let mut scheduler = SchedulerHandle::new(db, proxy);
        scheduler.start().await.expect("start scheduler");
        for _ in 0..50 {
            if scheduler.runtime_status().current_state == "probing" {
                break;
            }
            sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(scheduler.runtime_status().current_state, "probing");

        let result = timeout(Duration::from_millis(500), scheduler.pause()).await;

        assert!(
            result.is_ok(),
            "pause should not wait for the active probe timeout"
        );
        result.expect("pause returned").expect("pause ok");
        let status = scheduler.runtime_status();
        assert!(!status.running);
        assert_eq!(status.current_state, "paused");
    }

    #[tokio::test]
    async fn live_scheduler_start_probe_persist_and_pause_when_enabled() {
        if !live_tests_enabled() {
            return;
        }

        let dir = tempdir().expect("temp dir");
        let db =
            Arc::new(Database::open(dir.path().join("scheduler-live.sqlite3")).expect("open db"));
        db.migrate().expect("migrate db");
        db.save_profile(Profile {
            token: None,
            base_url: String::new(),
            model: String::new(),
            timeout_seconds: 8,
            min_interval_seconds: 60,
            max_interval_seconds: 120,
            start_time: "00:00".to_string(),
            end_time: "24:00".to_string(),
            ..Profile::default()
        })
        .expect("save profile");

        let proxy = Arc::new(tokio::sync::Mutex::new(ProxyHandle::new(
            ProxyConfig::default(),
            15800,
        )));
        let mut scheduler = SchedulerHandle::new(db.clone(), proxy);
        scheduler.start().await.expect("start scheduler");
        sleep(Duration::from_millis(250)).await;
        assert!(scheduler.runtime_status().running);

        scheduler.pause().await.expect("pause scheduler");
        let status = scheduler.runtime_status();
        assert!(!status.running);
        assert_eq!(status.current_state, "paused");

        let events = db.list_events(10, None).expect("list events");
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].status,
            ProbeStatus::Success | ProbeStatus::QueueMiss | ProbeStatus::Timeout
        ));
    }

    #[test]
    fn derive_status_preserves_active_probing_state() {
        let now = Local::now().to_rfc3339();
        let status = derive_status(
            "default".to_string(),
            RuntimeStatus {
                running: true,
                next_probe_at: None,
                current_state: "probing".to_string(),
                in_window: true,
            },
            Some(ProbeEventDto {
                id: "event-1".to_string(),
                profile_id: "default".to_string(),
                started_at: now.clone(),
                ended_at: now,
                duration_ms: 10,
                status: ProbeStatus::Unknown,
                error_kind: Some("unknown".to_string()),
                exit_code: None,
                base_url: String::new(),
                model: "claude-opus-4-8[1M]".to_string(),
                key_summary: None,
                prompt_summary: None,
                prompt_truncated: false,
                stdout_summary: None,
                stderr_summary: None,
                stdout_truncated: false,
                stderr_truncated: false,
                fingerprint_os: None,
                fingerprint_arch: None,
                fingerprint_device_id: None,
                fingerprint_device_id_status: None,
                fingerprint_session_id_status: None,
                fingerprint_context_management: None,
                fingerprint_source: None,
            }),
            None,
            0,
        );

        assert_eq!(status.current_state, "probing");
    }

    #[test]
    fn derive_status_preserves_runtime_config_error() {
        let status = derive_status(
            "default".to_string(),
            RuntimeStatus {
                running: false,
                next_probe_at: None,
                current_state: "config_error".to_string(),
                in_window: false,
            },
            None,
            None,
            0,
        );

        assert_eq!(status.current_state, "config_error");
    }
}
