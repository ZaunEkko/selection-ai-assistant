use tauri::{AppHandle, Emitter};

use crate::ai::action_classifier::{classify_action, AiAction};
use crate::selection::types::{SelectionCandidate, SelectionReadMethod};
use crate::types::{Point, PublicError};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelContext {
    pub selection: SelectionCandidate,
    pub action: AiAction,
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
    };
    let action = classify_action(&selection.text);

    Ok(PanelContext { selection, action })
}

#[tauri::command]
pub fn open_panel_for_text(app: AppHandle, text: String) -> Result<PanelContext, PublicError> {
    let context = create_panel_context_for_text(&text)?;

    app.emit("panel_context", &context)
        .map_err(|err| PublicError {
            code: "emit_failed".to_string(),
            message: err.to_string(),
        })?;

    Ok(context)
}
