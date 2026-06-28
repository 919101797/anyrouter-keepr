use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, Manager};

use crate::AppState;

pub fn setup_tray(app: &mut App) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("show", "显示窗口")
        .separator()
        .text("quit", "退出")
        .build()?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../../icons/icon.png"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("AnyRouter Keeper")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                crate::system::window::show_main_window(app);
            }
            "quit" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = state.db.flush_buffer();
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => {
                crate::system::window::show_main_window(tray.app_handle());
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}
