use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

use crate::{
    app_state::AppState,
    config::{AppConfig, CloseButtonBehavior},
};

const MAIN_WINDOW_LABEL: &str = "main";
pub const AUTOSTART_ARG: &str = "--autostart";
const TRAY_SHOW_SETTINGS_ID: &str = "show-settings";
const TRAY_QUIT_ID: &str = "quit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseRequestAction {
    Ignore,
    AskUser,
    MinimizeToTray,
    ExitApp,
}

pub fn is_autostart_launch<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter().any(|arg| arg.as_ref() == AUTOSTART_ARG)
}

pub fn should_show_main_window_on_startup(config: &AppConfig) -> bool {
    !config.start_minimized_to_tray
}

pub fn close_request_action_for_window(
    label: &str,
    behavior: CloseButtonBehavior,
) -> CloseRequestAction {
    if label != MAIN_WINDOW_LABEL {
        return CloseRequestAction::Ignore;
    }

    match behavior {
        CloseButtonBehavior::Ask => CloseRequestAction::AskUser,
        CloseButtonBehavior::MinimizeToTray => CloseRequestAction::MinimizeToTray,
        CloseButtonBehavior::ExitApp => CloseRequestAction::ExitApp,
    }
}

pub fn should_hide_to_background_on_close(label: &str) -> bool {
    close_request_action_for_window(label, CloseButtonBehavior::MinimizeToTray)
        == CloseRequestAction::MinimizeToTray
}

pub fn show_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

pub fn apply_startup_visibility(app: &tauri::App) -> tauri::Result<()> {
    let should_show = app
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .config
                .lock()
                .ok()
                .map(|config| should_show_main_window_on_startup(&config))
        })
        .unwrap_or(true);

    if should_show {
        show_main_window(app.handle())?;
    } else if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.hide()?;
    }

    Ok(())
}

pub fn apply_main_close_choice(
    app: &tauri::AppHandle,
    behavior: CloseButtonBehavior,
) -> tauri::Result<()> {
    match behavior {
        CloseButtonBehavior::Ask => {
            if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                window.emit("main_close_confirmation_requested", ())?;
            }
        }
        CloseButtonBehavior::MinimizeToTray => {
            if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                window.hide()?;
            }
        }
        CloseButtonBehavior::ExitApp => app.exit(0),
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
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    match configured_close_request_action(window) {
        CloseRequestAction::Ignore => {}
        CloseRequestAction::AskUser => {
            api.prevent_close();
            let _ = window.emit("main_close_confirmation_requested", ());
        }
        CloseRequestAction::MinimizeToTray => {
            api.prevent_close();
            let _ = window.hide();
        }
        CloseRequestAction::ExitApp => {
            api.prevent_close();
            window.app_handle().exit(0);
        }
    }
}

fn configured_close_request_action(window: &tauri::Window) -> CloseRequestAction {
    let behavior = window
        .try_state::<AppState>()
        .and_then(|state| {
            state
                .config
                .lock()
                .ok()
                .map(|config| config.close_button_behavior)
        })
        .unwrap_or_default();

    close_request_action_for_window(window.label(), behavior)
}
