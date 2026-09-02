pub mod commands;
pub mod core;
pub mod security;
pub mod storage;
pub mod system;

use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;

use crate::core::proxy::ProxyConfig;
use crate::core::proxy::ProxyHandle;
use crate::core::scheduler::SchedulerHandle;
use crate::storage::db::Database;

pub struct AppState {
    pub db: Arc<Database>,
    pub scheduler: Arc<Mutex<SchedulerHandle>>,
    pub proxy: Arc<Mutex<ProxyHandle>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        system::app_log::error("panic", info.to_string());
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            system::window::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let db = Arc::new(Database::open_default()?);
            db.migrate()?;
            let saved_target = db
                .load_proxy_target()
                .unwrap_or_else(|_| (String::from("Windows"), String::from("x64")));
            let proxy_config = ProxyConfig {
                target_os: saved_target.0,
                target_arch: saved_target.1,
                upstream_url: String::from("https://anyrouter.top"),
                dynamic_upstream: true,
            };
            let proxy = Arc::new(Mutex::new(ProxyHandle::new(proxy_config, 15800)));
            let scheduler = Arc::new(Mutex::new(SchedulerHandle::new(db.clone(), proxy.clone())));
            let should_restore_scheduler = db.is_enabled().unwrap_or(false);
            app.manage(AppState {
                db,
                scheduler: scheduler.clone(),
                proxy,
            });
            system::tray::setup_tray(app)?;
            if should_restore_scheduler {
                let scheduler = scheduler.clone();
                tauri::async_runtime::spawn(async move {
                    system::app_log::info("scheduler.restore", "enabled profile found");
                    let mut scheduler = scheduler.lock().await;
                    if let Err(error) = scheduler.start().await {
                        system::app_log::error("scheduler.restore", error);
                    }
                });
            }
            system::app_log::info("app", "started");
            system::app_log::info(
                "app",
                format!("log_file={}", system::app_log::path().display()),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::claude::get_claude_installation,
            commands::claude::get_claude_key_value,
            commands::claude::get_claude_runtime_config,
            commands::claude::get_upstream_models,
            commands::claude::refresh_claude_installation,
            commands::claude::test_claude_installation,
            commands::claude::list_claude_detection_logs,
            commands::fingerprint::delete_claude_fingerprint_history,
            commands::fingerprint::get_claude_fingerprint_snapshot,
            commands::fingerprint::regenerate_claude_fingerprint,
            commands::fingerprint::restore_claude_fingerprint,
            commands::fingerprint::switch_all_fingerprints,
            commands::profile::get_profile,
            commands::profile::save_profile,
            commands::probe::run_probe_now,
            commands::proxy::get_proxy_status,
            commands::proxy::set_proxy_target,
            commands::proxy::start_proxy,
            commands::proxy::stop_proxy,
            commands::scheduler::start_scheduler,
            commands::scheduler::pause_scheduler,
            commands::scheduler::get_current_status,
            commands::stats::list_probe_events,
            commands::stats::get_activity_summary,
            commands::stats::compact_storage,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                system::app_log::info(
                    "window.close_requested",
                    format!("label={} action=hide_to_background", window.label()),
                );
                if let Some(state) = window.try_state::<AppState>() {
                    let db = state.db.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = db.flush_buffer();
                    });
                }
                api.prevent_close();
                system::window::hide_to_background(window);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running AnyRouter Keeper");
}
