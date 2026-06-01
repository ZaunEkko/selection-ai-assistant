use tauri::{AppHandle, Manager};

use crate::floating_window::positioning::{place_near_anchor, ScreenBounds, WindowSize};
use crate::types::{Point, PublicError};

const FLOATING_BUTTON_SIZE: WindowSize = WindowSize {
    width: 44.0,
    height: 32.0,
};
const AI_PANEL_FALLBACK_SIZE: WindowSize = WindowSize {
    width: 420.0,
    height: 520.0,
};

fn command_error(code: &str, err: impl ToString) -> PublicError {
    PublicError {
        code: code.to_string(),
        message: err.to_string(),
    }
}

fn position_window_near_anchor(
    app: &AppHandle,
    anchor: Point,
    size: WindowSize,
) -> Result<Point, PublicError> {
    let screen = screen_bounds_for_anchor(app, anchor)?;
    Ok(place_near_anchor(anchor, size, screen))
}

fn screen_bounds_for_anchor(app: &AppHandle, anchor: Point) -> Result<ScreenBounds, PublicError> {
    let monitors = app
        .available_monitors()
        .map_err(|err| command_error("monitor_unavailable", err))?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            let x = position.x as f64;
            let y = position.y as f64;
            anchor.x >= x
                && anchor.x <= x + size.width as f64
                && anchor.y >= y
                && anchor.y <= y + size.height as f64
        })
        .or_else(|| monitors.first())
        .ok_or_else(|| PublicError {
            code: "monitor_unavailable".to_string(),
            message: "No monitor is available for window placement.".to_string(),
        })?;

    let position = monitor.position();
    let size = monitor.size();
    Ok(ScreenBounds {
        x: position.x as f64,
        y: position.y as f64,
        width: size.width as f64,
        height: size.height as f64,
    })
}

fn current_window_size(window: &tauri::WebviewWindow, fallback: WindowSize) -> WindowSize {
    window
        .outer_size()
        .map(|size| WindowSize {
            width: size.width as f64,
            height: size.height as f64,
        })
        .unwrap_or(fallback)
}

fn set_window_position(window: &tauri::WebviewWindow, position: Point) -> Result<(), PublicError> {
    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: position.x.round() as i32,
            y: position.y.round() as i32,
        }))
        .map_err(|err| command_error("set_position_failed", err))
}

#[tauri::command]
pub fn show_floating_button(app: AppHandle, position: Point) -> Result<(), PublicError> {
    let window = app
        .get_webview_window("floating-button")
        .ok_or_else(|| PublicError {
            code: "floating_button_missing".to_string(),
            message: "Floating button window is not configured.".to_string(),
        })?;

    let position = position_window_near_anchor(&app, position, FLOATING_BUTTON_SIZE)?;
    set_window_position(&window, position)?;
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
    let window = app
        .get_webview_window("ai-panel")
        .ok_or_else(|| PublicError {
            code: "ai_panel_missing".to_string(),
            message: "AI panel window is not configured.".to_string(),
        })?;

    let size = current_window_size(&window, AI_PANEL_FALLBACK_SIZE);
    let position = position_window_near_anchor(&app, position, size)?;
    set_window_position(&window, position)?;
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
