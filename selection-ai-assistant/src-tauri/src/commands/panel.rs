use tauri::{AppHandle, Emitter, Manager, State};

use crate::app_state::AppState;

use crate::floating_window::positioning::{
    place_near_anchor, place_source_left_of_panel, ScreenBounds, WindowSize,
};
use crate::types::{Point, PublicError, Rect};

const FLOATING_BUTTON_SIZE: WindowSize = WindowSize {
    width: 40.0,
    height: 40.0,
};
const AI_PANEL_FALLBACK_SIZE: WindowSize = WindowSize {
    width: 420.0,
    height: 520.0,
};
const SOURCE_TEXT_FALLBACK_SIZE: WindowSize = WindowSize {
    width: 360.0,
    height: 620.0,
};
const SOURCE_TEXT_WINDOW_GAP: f64 = 12.0;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTextContext {
    pub text: String,
}

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

fn position_source_text_window_left_of_panel(
    app: &AppHandle,
    source_size: WindowSize,
) -> Result<Point, PublicError> {
    let Some(panel) = app.get_webview_window("ai-panel") else {
        return position_window_near_anchor(app, Point { x: 200.0, y: 200.0 }, source_size);
    };

    let panel_position = panel
        .outer_position()
        .map_err(|err| command_error("panel_position_unavailable", err))?;
    let panel_size = current_window_size(&panel, AI_PANEL_FALLBACK_SIZE);
    let panel_anchor = Point {
        x: panel_position.x as f64 + panel_size.width / 2.0,
        y: panel_position.y as f64,
    };
    let screen = screen_bounds_for_anchor(app, panel_anchor)?;
    let layout = place_source_left_of_panel(
        Point {
            x: panel_position.x as f64,
            y: panel_position.y as f64,
        },
        panel_size,
        source_size,
        screen,
        SOURCE_TEXT_WINDOW_GAP,
    );

    set_window_position(&panel, layout.panel)?;
    Ok(layout.source)
}

fn assistant_window_rects(app: &AppHandle) -> Vec<Rect> {
    ["floating-button", "ai-panel", "source-text"]
        .into_iter()
        .filter_map(|label| app.get_webview_window(label))
        .filter(|window| window.is_visible().unwrap_or(false))
        .filter_map(|window| {
            let position = window.outer_position().ok()?;
            let size = window.outer_size().ok()?;
            Some(Rect {
                x: position.x as f64,
                y: position.y as f64,
                width: size.width as f64,
                height: size.height as f64,
            })
        })
        .collect()
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
    let assistant_rects = assistant_window_rects(&app);
    crate::input_monitor::notify_ai_panel_closed_by_user(assistant_rects);
    if let Some(window) = app.get_webview_window("ai-panel") {
        window
            .hide()
            .map_err(|err| command_error("hide_failed", err))?;
    }
    hide_source_text_window(app)?;
    Ok(())
}

#[tauri::command]
pub fn show_source_text_window(app: AppHandle, text: String) -> Result<(), PublicError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(PublicError {
            code: "source_text_required".to_string(),
            message: "打开原文窗口前需要选中文本。".to_string(),
        });
    }

    app.state::<AppState>()
        .store_latest_source_text(text.to_string());

    let window = app
        .get_webview_window("source-text")
        .ok_or_else(|| PublicError {
            code: "source_text_window_missing".to_string(),
            message: "Source text window is not configured.".to_string(),
        })?;

    let size = current_window_size(&window, SOURCE_TEXT_FALLBACK_SIZE);
    let position = position_source_text_window_left_of_panel(&app, size)?;
    set_window_position(&window, position)?;
    window
        .show()
        .map_err(|err| command_error("show_failed", err))?;
    window
        .set_focus()
        .map_err(|err| command_error("focus_failed", err))?;
    window
        .emit(
            "source_text_context",
            SourceTextContext {
                text: text.to_string(),
            },
        )
        .map_err(|err| command_error("emit_failed", err))?;
    Ok(())
}

#[tauri::command]
pub fn get_latest_source_text_context(state: State<AppState>) -> Option<SourceTextContext> {
    state
        .latest_source_text()
        .map(|text| SourceTextContext { text })
}

#[tauri::command]
pub fn hide_source_text_window(app: AppHandle) -> Result<(), PublicError> {
    if let Some(window) = app.get_webview_window("source-text") {
        window
            .hide()
            .map_err(|err| command_error("hide_failed", err))?;
    }
    Ok(())
}
