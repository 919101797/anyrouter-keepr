use tauri::{AppHandle, Manager, Window};

pub fn show_main_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);

    if let Some(window) = app.get_webview_window("main") {
        #[cfg(windows)]
        let _ = window.set_skip_taskbar(false);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_to_background(window: &Window) {
    #[cfg(windows)]
    let _ = window.set_skip_taskbar(true);
    let _ = window.hide();

    #[cfg(target_os = "macos")]
    let _ = window
        .app_handle()
        .set_activation_policy(tauri::ActivationPolicy::Accessory);
}
