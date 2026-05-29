use selection_ai_assistant_lib::ai::action_classifier::AiAction;
use selection_ai_assistant_lib::commands::selection::create_panel_context_for_text;
use selection_ai_assistant_lib::selection::types::SelectionReadMethod;

#[test]
fn creates_panel_context_for_valid_manual_text() {
    let context = create_panel_context_for_text(" hello world ").unwrap();

    assert_eq!(context.selection.text, "hello world");
    assert_eq!(context.selection.source_app, "manual");
    assert_eq!(context.selection.window_title, "Manual hotkey");
    assert_eq!(context.selection.read_method, SelectionReadMethod::HotkeyClipboard);
    assert_eq!(context.action, AiAction::TranslateExplain);
}

#[test]
fn rejects_panel_context_for_tiny_text() {
    let err = create_panel_context_for_text(" a ").unwrap_err();

    assert_eq!(err.code, "selection_text_too_short");
}
