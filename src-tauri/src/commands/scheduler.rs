use tauri::State;

use crate::core::scheduler::derive_status;
use crate::core::types::AppStatus;
use crate::AppState;

#[tauri::command]
pub async fn start_scheduler(state: State<'_, AppState>) -> Result<(), String> {
    let mut scheduler = state.scheduler.lock().await;
    scheduler.start().await
}

#[tauri::command]
pub async fn pause_scheduler(state: State<'_, AppState>) -> Result<(), String> {
    let mut scheduler = state.scheduler.lock().await;
    scheduler.pause().await
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
