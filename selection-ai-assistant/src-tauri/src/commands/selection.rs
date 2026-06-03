use tauri::{AppHandle, Emitter, Manager, State};

use crate::ai::action_classifier::{classify_action, AiAction};
use crate::app_state::AppState;
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
pub fn get_latest_panel_context(state: State<AppState>) -> Option<PanelContext> {
    state.latest_selection()
}

#[tauri::command]
pub fn open_panel_for_current_selection(
    app: AppHandle,
    state: State<AppState>,
) -> Result<PanelContext, PublicError> {
    let context = state.latest_selection().ok_or_else(|| PublicError {
        code: "selection_context_missing".to_string(),
        message: "No selected text is available. Select text first.".to_string(),
    })?;
    let opened = open_panel_for_context(&app, context)?;
    state.store_latest_selection(opened.clone());
    Ok(opened)
}
