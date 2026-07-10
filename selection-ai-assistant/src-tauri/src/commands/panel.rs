use tauri::{AppHandle, Emitter, Manager, State};

use crate::app_state::AppState;

use crate::floating_window::positioning::{
    place_near_anchor, place_source_left_of_panel, place_toolbar_above_anchor,
    place_toolbar_near_selection, place_translate_result_near_anchor,
    place_translate_result_near_selection, ScreenBounds, WindowSize,
};
use crate::types::{Point, PublicError, Rect};

const FLOATING_BUTTON_SIZE: WindowSize = WindowSize {
    width: 244.0,
    height: 44.0,
};
const REPLACEMENT_PRESET_COMPACT_SIZE: WindowSize = WindowSize {
    width: 420.0,
    height: 78.0,
};
const REPLACEMENT_PRESET_EXPANDED_SIZE: WindowSize = WindowSize {
    width: 420.0,
    height: 126.0,
};
const AI_PANEL_FALLBACK_SIZE: WindowSize = WindowSize {
    width: 420.0,
    height: 520.0,
};
const SOURCE_TEXT_FALLBACK_SIZE: WindowSize = WindowSize {
    width: 360.0,
    height: 620.0,
};
const TRANSLATE_RESULT_SIZE: WindowSize = WindowSize {
    width: 320.0,
    height: 180.0,
};
const SOURCE_TEXT_WINDOW_GAP: f64 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TargetPresetKind {
    Replacement,
    Translation,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetPresetContext {
    pub kind: TargetPresetKind,
}

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

fn position_toolbar_above_anchor(
    app: &AppHandle,
    anchor: Point,
    size: WindowSize,
) -> Result<Point, PublicError> {
    let screen = screen_bounds_for_anchor(app, anchor)?;
    Ok(place_toolbar_above_anchor(anchor, size, screen))
}

fn position_toolbar_near_selection(
    app: &AppHandle,
    anchor: Point,
    selection_rects: &[Rect],
    size: WindowSize,
) -> Result<Point, PublicError> {
    let screen = screen_bounds_for_anchor(app, anchor)?;
    Ok(place_toolbar_near_selection(
        anchor,
        selection_rects,
        size,
        screen,
    ))
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

fn show_replacement_preset_without_activation(
    window: &tauri::WebviewWindow,
) -> Result<(), PublicError> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};

        let hwnd = window
            .hwnd()
            .map_err(|err| command_error("window_handle_unavailable", err))?;
        unsafe {
            ShowWindow(hwnd.0, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        window
            .show()
            .map_err(|err| command_error("show_failed", err))
    }
}

fn hide_replacement_preset_window(window: &tauri::WebviewWindow) -> Result<(), PublicError> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

        let hwnd = window
            .hwnd()
            .map_err(|err| command_error("window_handle_unavailable", err))?;
        unsafe {
            ShowWindow(hwnd.0, SW_HIDE);
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        window
            .hide()
            .map_err(|err| command_error("hide_failed", err))
    }
}

fn position_replacement_preset_panel(
    app: &AppHandle,
    preset_size: WindowSize,
) -> Result<Point, PublicError> {
    let floating = app
        .get_webview_window("floating-button")
        .ok_or_else(|| PublicError {
            code: "floating_button_missing".to_string(),
            message: "Floating button window is not configured.".to_string(),
        })?;
    let floating_position = floating
        .outer_position()
        .map_err(|err| command_error("floating_position_unavailable", err))?;
    let floating_size = current_window_size(&floating, FLOATING_BUTTON_SIZE);
    let anchor = Point {
        x: floating_position.x as f64 + floating_size.width / 2.0,
        y: floating_position.y as f64 + floating_size.height / 2.0,
    };
    let screen = screen_bounds_for_anchor(app, anchor)?;
    let gap = 6.0;
    let min_x = screen.x;
    let min_y = screen.y;
    let max_x = (screen.x + screen.width - preset_size.width).max(min_x);
    let max_y = (screen.y + screen.height - preset_size.height).max(min_y);
    let floating_x = floating_position.x as f64;
    let floating_y = floating_position.y as f64;
    let above_y = floating_y - preset_size.height - gap;
    let below_y = floating_y + floating_size.height + gap;
    let y = if above_y >= screen.y {
        above_y
    } else {
        below_y
    };

    Ok(Point {
        x: floating_x.clamp(min_x, max_x),
        y: y.clamp(min_y, max_y),
    })
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
    [
        "floating-button",
        "replacement-preset",
        "ai-panel",
        "source-text",
        "translate-result",
        "screenshot-overlay",
    ]
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
    show_floating_button_for_selection(app, position, &[])
}

pub fn show_floating_button_for_selection(
    app: AppHandle,
    position: Point,
    selection_rects: &[Rect],
) -> Result<(), PublicError> {
    let window = app
        .get_webview_window("floating-button")
        .ok_or_else(|| PublicError {
            code: "floating_button_missing".to_string(),
            message: "Floating button window is not configured.".to_string(),
        })?;

    let position = floating_button_position_for_selection(&app, position, selection_rects)?;
    show_floating_button_window(&window, position)
}

pub fn floating_button_position_for_selection(
    app: &AppHandle,
    position: Point,
    selection_rects: &[Rect],
) -> Result<Point, PublicError> {
    if selection_rects.is_empty() {
        position_toolbar_above_anchor(app, position, FLOATING_BUTTON_SIZE)
    } else {
        position_toolbar_near_selection(app, position, selection_rects, FLOATING_BUTTON_SIZE)
    }
}

pub fn show_floating_button_at_position(
    app: AppHandle,
    position: Point,
) -> Result<(), PublicError> {
    let window = app
        .get_webview_window("floating-button")
        .ok_or_else(|| PublicError {
            code: "floating_button_missing".to_string(),
            message: "Floating button window is not configured.".to_string(),
        })?;

    show_floating_button_window(&window, position)
}

fn show_floating_button_window(
    window: &tauri::WebviewWindow,
    position: Point,
) -> Result<(), PublicError> {
    set_window_position(window, position)?;
    window
        .show()
        .map_err(|err| command_error("show_failed", err))?;
    Ok(())
}

#[tauri::command]
pub fn hide_floating_button(app: AppHandle) -> Result<(), PublicError> {
    #[cfg(debug_assertions)]
    eprintln!("[panel] hide_floating_button");
    let floating_result = if let Some(window) = app.get_webview_window("floating-button") {
        window
            .hide()
            .map_err(|err| command_error("hide_failed", err))
    } else {
        Ok(())
    };
    let preset_result = hide_replacement_preset_panel(app);
    floating_result?;
    preset_result
}

#[tauri::command]
pub fn show_replacement_preset_panel(
    app: AppHandle,
    kind: TargetPresetKind,
) -> Result<(), PublicError> {
    let floating = app
        .get_webview_window("floating-button")
        .ok_or_else(|| PublicError {
            code: "floating_button_missing".to_string(),
            message: "Floating button window is not configured.".to_string(),
        })?;
    if !floating.is_visible().unwrap_or(false) {
        #[cfg(debug_assertions)]
        eprintln!("[panel] skip preset show because floating button is hidden");
        return Ok(());
    }

    #[cfg(debug_assertions)]
    eprintln!("[panel] show replacement preset: kind={kind:?}");

    let window = app
        .get_webview_window("replacement-preset")
        .ok_or_else(|| PublicError {
            code: "replacement_preset_missing".to_string(),
            message: "Replacement preset window is not configured.".to_string(),
        })?;

    let preset_size = current_window_size(&window, REPLACEMENT_PRESET_COMPACT_SIZE);
    let position = position_replacement_preset_panel(&app, preset_size)?;
    set_window_position(&window, position)?;
    show_replacement_preset_without_activation(&window)?;
    if !floating.is_visible().unwrap_or(false) {
        hide_replacement_preset_window(&window)?;
        return Ok(());
    }
    window
        .emit("target_preset_context", TargetPresetContext { kind })
        .map_err(|err| command_error("emit_failed", err))?;
    Ok(())
}

#[tauri::command]
pub fn set_replacement_preset_panel_expanded(
    app: AppHandle,
    expanded: bool,
) -> Result<(), PublicError> {
    let window = app
        .get_webview_window("replacement-preset")
        .ok_or_else(|| PublicError {
            code: "replacement_preset_missing".to_string(),
            message: "Replacement preset window is not configured.".to_string(),
        })?;
    let requested_size = if expanded {
        REPLACEMENT_PRESET_EXPANDED_SIZE
    } else {
        REPLACEMENT_PRESET_COMPACT_SIZE
    };
    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize {
            width: requested_size.width,
            height: requested_size.height,
        }))
        .map_err(|err| command_error("set_size_failed", err))?;
    let actual_size = current_window_size(&window, requested_size);
    let position = position_replacement_preset_panel(&app, actual_size)?;
    set_window_position(&window, position)
}

#[tauri::command]
pub fn focus_floating_button(app: AppHandle) -> Result<(), PublicError> {
    if let Some(window) = app.get_webview_window("floating-button") {
        window
            .set_focus()
            .map_err(|err| command_error("focus_failed", err))?;
    }
    Ok(())
}

#[tauri::command]
pub fn hide_replacement_preset_panel(app: AppHandle) -> Result<(), PublicError> {
    #[cfg(debug_assertions)]
    eprintln!("[panel] hide_replacement_preset_panel");
    let hide_result = if let Some(window) = app.get_webview_window("replacement-preset") {
        hide_replacement_preset_window(&window)
    } else {
        Ok(())
    };
    if let Some(floating) = app.get_webview_window("floating-button") {
        let _ = floating.emit("target_preset_panel_hidden", ());
    }
    hide_result
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

#[tauri::command]
pub fn show_translate_result(
    app: AppHandle,
    position: Point,
    original_text: String,
    translated_text: String,
    selection_rects: Vec<Rect>,
) -> Result<(), PublicError> {
    let window = app
        .get_webview_window("translate-result")
        .ok_or_else(|| PublicError {
            code: "translate_result_missing".to_string(),
            message: "Translate result window is not configured.".to_string(),
        })?;

    let position = {
        let screen = screen_bounds_for_anchor(&app, position)?;
        let size = current_window_size(&window, TRANSLATE_RESULT_SIZE);
        if selection_rects.is_empty() {
            place_translate_result_near_anchor(position, size, screen)
        } else {
            place_translate_result_near_selection(position, &selection_rects, size, screen)
        }
    };
    set_window_position(&window, position)?;
    window
        .show()
        .map_err(|err| command_error("show_failed", err))?;
    window
        .emit(
            "translate_result",
            TranslateResultPayload {
                original_text,
                translated_text,
            },
        )
        .map_err(|err| command_error("emit_failed", err))?;
    Ok(())
}

#[tauri::command]
pub fn hide_translate_result(app: AppHandle) -> Result<(), PublicError> {
    if let Some(window) = app.get_webview_window("translate-result") {
        window
            .hide()
            .map_err(|err| command_error("hide_failed", err))?;
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResultPayload {
    pub original_text: String,
    pub translated_text: String,
}
