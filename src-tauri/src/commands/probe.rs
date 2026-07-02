use tauri::State;

use crate::core::claude_runner::{build_probe_fingerprint, run_probe};
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
    let (proxy_url, fingerprint) = {
        let proxy = state.proxy.lock().await;
        let status = proxy.status();
        (proxy.proxy_url(), build_probe_fingerprint(Some(&status)))
    };
    let mut event = run_probe(&profile, proxy_url.as_deref()).await;
    event.attach_fingerprint(fingerprint);
    let dto = ProbeEventDto::from(event.clone());
    app_log::info(
        "run_probe_now",
        format!(
            "proxy={} status={}",
            proxy_url.as_deref().unwrap_or("off"),
            dto.status.as_str(),
        ),
    );
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
    Ok(dto)
}
