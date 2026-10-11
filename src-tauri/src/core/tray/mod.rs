use crate::commands::{ActiveOperationGuard, ActiveUpdateGuard};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub fn setup_system_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let open_i = MenuItemBuilder::with_id("open", "Open Stardew Sync").build(app)?;
    let check_updates_i = MenuItemBuilder::with_id("check_updates", "Check for Updates...").build(app)?;
    let settings_i = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let quit_i = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&open_i)
        .separator()
        .item(&check_updates_i)
        .item(&settings_i)
        .separator()
        .item(&quit_i)
        .build()?;

    let icon = match app.default_window_icon() {
        Some(ic) => ic.clone(),
        None => return Ok(()),
    };

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .icon(icon)
        .tooltip("Stardew Sync")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "open" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }
                "check_updates" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                        let _ = window.emit("navigate-tab", "updates");
                    }
                }
                "settings" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                        let _ = window.emit("navigate-tab", "settings");
                    }
                }
                "quit" => {
                    // Safety Guard: Prevent quitting while an update or save operation is running
                    if ActiveUpdateGuard::is_active() || ActiveOperationGuard::is_active() {
                        eprintln!("[stardew-sync] Quit request blocked: active update or save operation in progress.");
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                            let _ = window.emit("operation-busy-warning", "Cannot exit while an operation is actively in progress.");
                        }
                        return;
                    }
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            } else if let TrayIconEvent::DoubleClick { .. } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
