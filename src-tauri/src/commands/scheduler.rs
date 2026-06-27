use tauri::State;

use crate::core::scheduler::derive_status;
use crate::core::types::AppStatus;
use crate::system::app_log;
use crate::AppState;

#[tauri::command]
pub async fn start_scheduler(state: State<'_, AppState>) -> Result<(), String> {
    app_log::info("start_scheduler", "requested");
    let mut scheduler = state.scheduler.lock().await;
    match scheduler.start().await {
        Ok(()) => {
            app_log::info("start_scheduler", "ok");
            Ok(())
        }
        Err(error) => {
            app_log::error("start_scheduler", &error);
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn pause_scheduler(state: State<'_, AppState>) -> Result<(), String> {
    app_log::info("pause_scheduler", "requested");
    let mut scheduler = state.scheduler.lock().await;
    match scheduler.pause().await {
        Ok(()) => {
            app_log::info("pause_scheduler", "ok");
            Ok(())
        }
        Err(error) => {
            app_log::error("pause_scheduler", &error);
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn get_current_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    let runtime = {
        let scheduler = state.scheduler.lock().await;
        scheduler.runtime_status()
    };
    let last_event = state.db.last_event().map_err(|err| err.to_string())?;
    let last_success_at = state.db.last_success_at().map_err(|err| err.to_string())?;
    let consecutive_queue_miss = state
        .db
        .consecutive_queue_miss()
        .map_err(|err| err.to_string())?;

    Ok(derive_status(
        "default".to_string(),
        runtime,
        last_event,
        last_success_at,
        consecutive_queue_miss,
    ))
}
