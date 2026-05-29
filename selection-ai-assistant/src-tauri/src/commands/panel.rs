use tauri::{AppHandle, Manager};

use crate::types::{Point, PublicError};

fn command_error(code: &str, err: impl ToString) -> PublicError {
    PublicError {
        code: code.to_string(),
        message: err.to_string(),
    }
}

#[tauri::command]
pub fn show_floating_button(app: AppHandle, position: Point) -> Result<(), PublicError> {
    let window = app
        .get_webview_window("floating-button")
        .ok_or_else(|| PublicError {
            code: "floating_button_missing".to_string(),
            message: "Floating button window is not configured.".to_string(),
        })?;

    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: position.x.round() as i32,
            y: position.y.round() as i32,
        }))
        .map_err(|err| command_error("set_position_failed", err))?;
    window
        .show()
        .map_err(|err| command_error("show_failed", err))?;
    Ok(())
}

#[tauri::command]
pub fn hide_floating_button(app: AppHandle) -> Result<(), PublicError> {
    if let Some(window) = app.get_webview_window("floating-button") {
        window
            .hide()
            .map_err(|err| command_error("hide_failed", err))?;
    }
    Ok(())
}

#[tauri::command]
pub fn show_ai_panel(app: AppHandle, position: Point) -> Result<(), PublicError> {
    let window = app.get_webview_window("ai-panel").ok_or_else(|| PublicError {
        code: "ai_panel_missing".to_string(),
        message: "AI panel window is not configured.".to_string(),
    })?;

    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: position.x.round() as i32,
            y: position.y.round() as i32,
        }))
        .map_err(|err| command_error("set_position_failed", err))?;
    window
        .show()
        .map_err(|err| command_error("show_failed", err))?;
    window
        .set_focus()
        .map_err(|err| command_error("focus_failed", err))?;
    Ok(())
}

#[tauri::command]
pub fn hide_ai_panel(app: AppHandle) -> Result<(), PublicError> {
    if let Some(window) = app.get_webview_window("ai-panel") {
        window
            .hide()
            .map_err(|err| command_error("hide_failed", err))?;
    }
    Ok(())
}
