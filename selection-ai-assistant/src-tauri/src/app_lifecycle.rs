use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_SHOW_SETTINGS_ID: &str = "show-settings";
const TRAY_QUIT_ID: &str = "quit";

pub fn should_hide_to_background_on_close(label: &str) -> bool {
    label == MAIN_WINDOW_LABEL
}

pub fn show_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

pub fn setup_background_lifecycle(app: &tauri::App) -> tauri::Result<()> {
    let show_settings = MenuItemBuilder::with_id(TRAY_SHOW_SETTINGS_ID, "打开设置").build(app)?;
    let quit = MenuItemBuilder::with_id(TRAY_QUIT_ID, "退出").build(app)?;
    let tray_menu = MenuBuilder::new(app)
        .items(&[&show_settings, &quit])
        .build()?;

    let mut tray = TrayIconBuilder::with_id("selection-ai-assistant")
        .tooltip("Selection AI Assistant 正在后台运行")
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW_SETTINGS_ID => {
                let _ = show_main_window(app);
            }
            TRAY_QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

pub fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if should_hide_to_background_on_close(window.label()) {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
        }
    }
}
