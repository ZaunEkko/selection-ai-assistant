use selection_ai_assistant_lib::ai::action_classifier::AiAction;
use selection_ai_assistant_lib::commands::selection::{
    create_panel_context_for_selection, create_panel_context_for_text,
};
use selection_ai_assistant_lib::selection::types::{SelectionCandidate, SelectionReadMethod};
use selection_ai_assistant_lib::types::Point;

#[test]
fn creates_panel_context_for_valid_manual_text() {
    let context = create_panel_context_for_text(" hello world ").unwrap();

    assert_eq!(context.selection.text, "hello world");
    assert_eq!(context.selection.source_app, "manual");
    assert_eq!(context.selection.window_title, "Manual hotkey");
    assert_eq!(
        context.selection.read_method,
        SelectionReadMethod::HotkeyClipboard
    );
    assert_eq!(context.action, AiAction::TranslateExplain);
    assert!(!context.auto_run);
}

#[test]
fn creates_auto_run_context_for_selection_candidate() {
    let selection = SelectionCandidate::from_clipboard_text(
        "selected text".to_string(),
        "unknown".to_string(),
        "Unknown window".to_string(),
        Point { x: 10.0, y: 20.0 },
    );

    let context = create_panel_context_for_selection(selection.clone(), true).unwrap();

    assert_eq!(context.selection, selection);
    assert_eq!(context.action, AiAction::TranslateExplain);
    assert!(context.auto_run);
}

#[test]
fn rejects_panel_context_for_tiny_text() {
    let err = create_panel_context_for_text(" a ").unwrap_err();

    assert_eq!(err.code, "selection_text_too_short");
}
