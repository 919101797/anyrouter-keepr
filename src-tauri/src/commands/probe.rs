use tauri::State;

use crate::core::claude_runner::run_probe;
use crate::core::types::ProbeEventDto;
use crate::AppState;

#[tauri::command]
pub async fn run_probe_now(state: State<'_, AppState>) -> Result<ProbeEventDto, String> {
    let profile = state
        .db
        .get_runtime_profile()
        .map_err(|err| err.to_string())?;
    let event = run_probe(&profile).await;
    let dto = ProbeEventDto::from(event.clone());
    state.db.push_event(event).map_err(|err| err.to_string())?;
    state.db.flush_buffer().map_err(|err| err.to_string())?;
    Ok(dto)
}
