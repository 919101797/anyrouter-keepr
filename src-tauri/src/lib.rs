pub mod commands;
pub mod core;
pub mod security;
pub mod storage;
pub mod system;

use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;

use crate::core::scheduler::SchedulerHandle;
use crate::storage::db::Database;

pub struct AppState {
    pub db: Arc<Database>,
    pub scheduler: Arc<Mutex<SchedulerHandle>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            let scheduler = Arc::new(Mutex::new(SchedulerHandle::new(db.clone())));
            app.manage(AppState { db, scheduler });
            system::tray::setup_tray(app)?;
            system::app_log::info("app", "started");
            system::app_log::info(
                "app",
                format!("log_file={}", system::app_log::path().display()),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::claude::get_claude_installation,
            commands::claude::get_claude_runtime_config,
            commands::claude::refresh_claude_installation,
            commands::claude::list_claude_detection_logs,
            commands::profile::get_profile,
            commands::profile::save_profile,
            commands::probe::run_probe_now,
            commands::scheduler::start_scheduler,
            commands::scheduler::pause_scheduler,
            commands::scheduler::get_current_status,
            commands::stats::list_probe_events,
            commands::stats::get_activity_summary,
            commands::stats::compact_storage,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
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
