use tauri::State;

use crate::core::claude_runner::run_probe;
use crate::core::types::ProbeEventDto;
use crate::system::app_log;
use crate::AppState;

#[tauri::command]
pub async fn run_probe_now(state: State<'_, AppState>) -> Result<ProbeEventDto, String> {
    app_log::info("run_probe_now", "requested");
    let profile = state.db.get_runtime_profile().map_err(|err| {
        let message = err.to_string();
        app_log::error("run_probe_now.get_profile", &message);
        message
    })?;
    let event = run_probe(&profile).await;
    let dto = ProbeEventDto::from(event.clone());
    state.db.push_event(event).map_err(|err| {
        let message = err.to_string();
        app_log::error("run_probe_now.push_event", &message);
        message
    })?;
    state.db.flush_buffer().map_err(|err| {
        let message = err.to_string();
        app_log::error("run_probe_now.flush_buffer", &message);
        message
    })?;
    app_log::info(
        "run_probe_now",
        format!(
            "ok status={} error={:?}",
            dto.status.as_str(),
            dto.error_kind
        ),
    );
    Ok(dto)
}
