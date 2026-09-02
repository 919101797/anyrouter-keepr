use tauri::State;

use crate::core::direct_claude_route::{
    disable_direct_claude_route, resolve_proxy_upstream_url, should_follow_cc_switch,
    sync_direct_claude_route,
};
use crate::core::proxy::{ProxyConfig, ProxyStatus};
use crate::AppState;

const DEFAULT_PROXY_PORT: u16 = 15800;

#[tauri::command]
pub async fn start_proxy(state: State<'_, AppState>) -> Result<ProxyStatus, String> {
    let mut proxy = state.proxy.lock().await;
    let profile = state
        .db
        .get_profile()
        .map_err(|err| format!("failed to read profile: {err}"))?;

    let current = proxy.status();
    let follow_cc_switch = should_follow_cc_switch(&profile.base_url, DEFAULT_PROXY_PORT);
    let upstream = resolve_proxy_upstream_url(
        &profile.base_url,
        &current.upstream_url,
        follow_cc_switch,
        DEFAULT_PROXY_PORT,
    );

    let config = ProxyConfig {
        target_os: current.target_os,
        target_arch: current.target_arch,
        upstream_url: upstream.upstream_url,
        dynamic_upstream: follow_cc_switch,
    };

    proxy.update_config(config.clone());
    proxy.start().await?;
    let direct_route = sync_direct_claude_route(DEFAULT_PROXY_PORT);
    if let Some(error) = direct_route.error.as_deref() {
        crate::system::app_log::error("proxy.direct_route", error);
    }

    crate::system::app_log::info(
        "proxy",
        format!(
            "started on 127.0.0.1:{}, rewriting OS={} Arch={} upstream={} source={} direct_route_enabled={}",
            DEFAULT_PROXY_PORT,
            config.target_os,
            config.target_arch,
            config.upstream_url,
            upstream.source,
            direct_route.enabled
        ),
    );

    let mut status = proxy.status();
    status.error = direct_route.error;
    Ok(status)
}

#[tauri::command]
pub async fn stop_proxy(state: State<'_, AppState>) -> Result<ProxyStatus, String> {
    let mut proxy = state.proxy.lock().await;
    proxy.stop();
    let direct_route = disable_direct_claude_route(DEFAULT_PROXY_PORT);
    if let Some(error) = direct_route.error.as_deref() {
        crate::system::app_log::error("proxy.direct_route_restore", error);
    }
    let mut status = proxy.status();
    status.error = direct_route.error;
    Ok(status)
}

#[tauri::command]
pub async fn get_proxy_status(state: State<'_, AppState>) -> Result<ProxyStatus, String> {
    let proxy = state.proxy.lock().await;
    Ok(proxy.status())
}

#[tauri::command]
pub async fn set_proxy_target(
    target_os: String,
    target_arch: String,
    state: State<'_, AppState>,
) -> Result<ProxyStatus, String> {
    let mut proxy = state.proxy.lock().await;
    let current = proxy.status();
    let config = ProxyConfig {
        target_os: if target_os.trim().is_empty() {
            current.target_os
        } else {
            target_os.trim().to_string()
        },
        target_arch: if target_arch.trim().is_empty() {
            current.target_arch
        } else {
            target_arch.trim().to_string()
        },
        upstream_url: current.upstream_url,
        dynamic_upstream: current.dynamic_upstream,
    };
    proxy.update_config(config);

    let s = proxy.status();
    state
        .db
        .save_proxy_target(&s.target_os, &s.target_arch)
        .map_err(|err| format!("failed to save proxy target: {err}"))?;

    Ok(s)
}
