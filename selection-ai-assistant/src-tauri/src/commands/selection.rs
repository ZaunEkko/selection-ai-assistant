use std::{thread, time::Duration};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

use crate::ai::action_classifier::{classify_action, AiAction};
use crate::app_state::AppState;
use crate::commands::access::require_webview_label;
use crate::commands::panel::{hide_floating_button, show_ai_panel};
use crate::selection::types::{SelectionAnchorSource, SelectionCandidate, SelectionReadMethod};
use crate::types::{Point, PublicError};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelContext {
    pub selection: SelectionCandidate,
    pub action: AiAction,
    #[serde(default)]
    pub auto_run: bool,
}

pub fn create_panel_context_for_selection(
    selection: SelectionCandidate,
    auto_run: bool,
) -> Result<PanelContext, PublicError> {
    let trimmed = selection.text.trim().to_string();
    if trimmed.chars().count() < 2 {
        return Err(PublicError {
            code: "selection_text_too_short".to_string(),
            message: "Selected text is too short.".to_string(),
        });
    }

    let mut selection = selection;
    if selection.text != trimmed {
        selection.text = trimmed.to_string();
    }
    let action = classify_action(&selection.text);

    Ok(PanelContext {
        selection,
        action,
        auto_run,
    })
}

pub fn create_panel_context_for_text(text: &str) -> Result<PanelContext, PublicError> {
    let trimmed = text.trim();
    if trimmed.chars().count() < 2 {
        return Err(PublicError {
            code: "selection_text_too_short".to_string(),
            message: "Selected text is too short.".to_string(),
        });
    }

    let selection = SelectionCandidate {
        id: uuid::Uuid::new_v4().to_string(),
        text: trimmed.to_string(),
        source_app: "manual".to_string(),
        window_title: "Manual hotkey".to_string(),
        anchor_rect: None,
        fallback_point: Point { x: 200.0, y: 200.0 },
        read_method: SelectionReadMethod::HotkeyClipboard,
        selection_rects: Vec::new(),
        explicit_anchor: None,
        anchor_source: Some(SelectionAnchorSource::HotkeyCursorFallback),
    };
    create_panel_context_for_selection(selection, false)
}

pub fn emit_panel_context(app: &AppHandle, context: &PanelContext) -> Result<(), PublicError> {
    app.state::<AppState>()
        .store_latest_source_text(context.selection.text.clone());
    app.emit(
        "source_text_context",
        crate::commands::panel::SourceTextContext {
            text: context.selection.text.clone(),
        },
    )
    .map_err(|err| PublicError {
        code: "emit_failed".to_string(),
        message: err.to_string(),
    })?;
    app.emit("panel_context", context)
        .map_err(|err| PublicError {
            code: "emit_failed".to_string(),
            message: err.to_string(),
        })
}

pub fn panel_context_for_visible_refresh(context: &PanelContext) -> PanelContext {
    let mut refreshed = context.clone();
    refreshed.auto_run = false;
    refreshed
}

pub fn open_panel_for_context(
    app: &AppHandle,
    mut context: PanelContext,
) -> Result<PanelContext, PublicError> {
    context.auto_run = true;
    show_ai_panel(app.clone(), context.selection.anchor_point())?;
    hide_floating_button(app.clone())?;
    emit_panel_context(app, &context)?;
    Ok(context)
}

#[tauri::command]
pub fn open_panel_for_text(app: AppHandle, text: String) -> Result<PanelContext, PublicError> {
    let context = create_panel_context_for_text(&text)?;

    emit_panel_context(&app, &context)?;

    Ok(context)
}

#[tauri::command]
pub fn get_latest_panel_context(
    webview: WebviewWindow,
    state: State<AppState>,
) -> Result<Option<PanelContext>, PublicError> {
    require_webview_label(&webview, &["floating-button", "ai-panel"])?;
    Ok(state.latest_selection())
}

#[tauri::command]
pub fn open_panel_for_current_selection(
    webview: WebviewWindow,
    app: AppHandle,
    state: State<AppState>,
) -> Result<PanelContext, PublicError> {
    require_webview_label(&webview, &["floating-button"])?;
    let context = state.latest_selection().ok_or_else(|| PublicError {
        code: "selection_context_missing".to_string(),
        message: "No selected text is available. Select text first.".to_string(),
    })?;
    let opened = open_panel_for_context(&app, context)?;
    state.store_latest_selection(opened.clone());
    Ok(opened)
}

#[tauri::command]
pub fn copy_to_clipboard(
    webview: WebviewWindow,
    app: AppHandle,
    text: String,
) -> Result<(), PublicError> {
    require_webview_label(&webview, &["ai-panel"])?;
    write_text_to_clipboard(&text)?;
    hide_floating_button(app)?;
    Ok(())
}

#[tauri::command]
pub fn replace_selected_text(
    webview: WebviewWindow,
    app: AppHandle,
    text: String,
    selection_id: Option<String>,
) -> Result<(), PublicError> {
    require_webview_label(&webview, &["floating-button"])?;
    let text = validate_replacement_text(&text)?.to_string();
    let state = app.state::<AppState>();
    validate_replacement_selection(&state, selection_id.as_deref())?;
    let target_window = state.latest_selection_window_handle();
    write_text_to_clipboard(&text)?;
    hide_floating_button(app)?;
    thread::sleep(Duration::from_millis(80));
    paste_clipboard_into_target_window(target_window)
}

pub fn validate_replacement_selection(
    state: &AppState,
    selection_id: Option<&str>,
) -> Result<(), PublicError> {
    let Some(selection_id) = selection_id.filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };

    let latest = state.latest_selection().ok_or_else(|| PublicError {
        code: "selection_context_missing".to_string(),
        message: "没有可替换的选区上下文。".to_string(),
    })?;

    if latest.selection.id == selection_id {
        Ok(())
    } else {
        Err(PublicError {
            code: "selection_context_changed".to_string(),
            message: "选区已经变化，请重新选择后再替换。".to_string(),
        })
    }
}

pub fn validate_replacement_text(text: &str) -> Result<&str, PublicError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        Err(PublicError {
            code: "replacement_text_required".to_string(),
            message: "替换文本不能为空。".to_string(),
        })
    } else {
        Ok(text)
    }
}

#[cfg(windows)]
fn write_text_to_clipboard(text: &str) -> Result<(), PublicError> {
    use std::ptr::null_mut;
    use windows_sys::Win32::System::{
        DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
        Ole::CF_UNICODETEXT,
    };

    let text_wide: Vec<u16> = text.encode_utf16().collect();
    let text_wide_with_null: Vec<u16> = {
        let mut v = text_wide;
        v.push(0);
        v
    };

    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            return Err(PublicError {
                code: "clipboard_open_failed".to_string(),
                message: "Failed to open clipboard.".to_string(),
            });
        }

        EmptyClipboard();

        let size = text_wide_with_null.len() * 2;
        let mem = GlobalAlloc(GMEM_MOVEABLE, size);
        if mem.is_null() {
            CloseClipboard();
            return Err(PublicError {
                code: "clipboard_alloc_failed".to_string(),
                message: "Failed to allocate clipboard memory.".to_string(),
            });
        }

        let ptr = GlobalLock(mem);
        if ptr.is_null() {
            CloseClipboard();
            return Err(PublicError {
                code: "clipboard_lock_failed".to_string(),
                message: "Failed to lock clipboard memory.".to_string(),
            });
        }

        std::ptr::copy_nonoverlapping(
            text_wide_with_null.as_ptr(),
            ptr as *mut u16,
            text_wide_with_null.len(),
        );
        GlobalUnlock(mem);

        if SetClipboardData(CF_UNICODETEXT.into(), mem).is_null() {
            CloseClipboard();
            return Err(PublicError {
                code: "clipboard_set_failed".to_string(),
                message: "Failed to set clipboard data.".to_string(),
            });
        }

        CloseClipboard();
    }

    Ok(())
}

#[cfg(not(windows))]
fn write_text_to_clipboard(_text: &str) -> Result<(), PublicError> {
    Err(PublicError {
        code: "clipboard_unsupported".to_string(),
        message: "当前平台暂不支持写入剪贴板。".to_string(),
    })
}

#[cfg(windows)]
fn paste_clipboard_into_target_window(target_window: Option<isize>) -> Result<(), PublicError> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::{
        Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
            VK_V,
        },
        WindowsAndMessaging::{GetForegroundWindow, IsWindow, SetForegroundWindow},
    };

    fn keyboard_input(vk: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let Some(target_window) = target_window else {
        return Err(PublicError {
            code: "selection_target_missing".to_string(),
            message: "没有可恢复焦点的原始窗口，请重新选择后再替换。".to_string(),
        });
    };

    let hwnd = target_window as HWND;
    let focused = unsafe {
        if IsWindow(hwnd) == 0 || SetForegroundWindow(hwnd) == 0 {
            false
        } else {
            thread::sleep(Duration::from_millis(80));
            GetForegroundWindow() == hwnd
        }
    };
    if !focused {
        return Err(PublicError {
            code: "selection_target_focus_failed".to_string(),
            message: "无法恢复原始窗口焦点，已取消自动替换。".to_string(),
        });
    }

    let mut inputs = [
        keyboard_input(VK_CONTROL, 0),
        keyboard_input(VK_V, 0),
        keyboard_input(VK_V, KEYEVENTF_KEYUP),
        keyboard_input(VK_CONTROL, KEYEVENTF_KEYUP),
    ];

    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        return Err(PublicError {
            code: "paste_failed".to_string(),
            message: "模拟粘贴替换选区失败。".to_string(),
        });
    }

    Ok(())
}

#[cfg(not(windows))]
fn paste_clipboard_into_target_window(_target_window: Option<isize>) -> Result<(), PublicError> {
    Err(PublicError {
        code: "paste_unsupported".to_string(),
        message: "当前平台暂不支持自动替换选区。".to_string(),
    })
}
