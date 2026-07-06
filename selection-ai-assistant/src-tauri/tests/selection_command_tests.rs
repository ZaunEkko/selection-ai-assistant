use selection_ai_assistant_lib::ai::action_classifier::AiAction;
use selection_ai_assistant_lib::app_state::AppState;
use selection_ai_assistant_lib::commands::selection::{
    create_panel_context_for_selection, create_panel_context_for_text,
    panel_context_for_visible_refresh, validate_replacement_selection,
};
use selection_ai_assistant_lib::config::AppConfig;
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
fn visible_panel_refresh_never_preserves_auto_run() {
    let selection = SelectionCandidate::from_clipboard_text(
        "selected text".to_string(),
        "unknown".to_string(),
        "Unknown window".to_string(),
        Point { x: 10.0, y: 20.0 },
    );
    let context = create_panel_context_for_selection(selection, true).unwrap();

    let refreshed = panel_context_for_visible_refresh(&context);

    assert!(!refreshed.auto_run);
    assert_eq!(refreshed.selection.text, "selected text");
    assert_eq!(refreshed.action, context.action);
}

#[test]
fn storing_latest_selection_also_updates_latest_source_text() {
    let state = AppState::new(AppConfig::default());
    let selection = SelectionCandidate::from_clipboard_text(
        "selected text".to_string(),
        "unknown".to_string(),
        "Unknown window".to_string(),
        Point { x: 10.0, y: 20.0 },
    );
    let context = create_panel_context_for_selection(selection, false).unwrap();

    state.store_latest_selection(context);

    assert_eq!(state.latest_source_text().as_deref(), Some("selected text"));
}

#[test]
fn clears_latest_selection_window_handle_with_selection_context() {
    let state = AppState::new(AppConfig::default());

    state.store_latest_selection_window_handle(42);
    assert_eq!(state.latest_selection_window_handle(), Some(42));

    state.clear_latest_selection();

    assert_eq!(state.latest_selection_window_handle(), None);
}

#[test]
fn replacement_selection_validation_rejects_stale_selection_id() {
    let state = AppState::new(AppConfig::default());
    let selection = SelectionCandidate::from_clipboard_text(
        "selected text".to_string(),
        "unknown".to_string(),
        "Unknown window".to_string(),
        Point { x: 10.0, y: 20.0 },
    );
    let context = create_panel_context_for_selection(selection, false).unwrap();
    let current_id = context.selection.id.clone();
    state.store_latest_selection(context);

    assert!(validate_replacement_selection(&state, Some(&current_id)).is_ok());
    let err = validate_replacement_selection(&state, Some("old-selection")).unwrap_err();

    assert_eq!(err.code, "selection_context_changed");
}

#[test]
fn rejects_panel_context_for_tiny_text() {
    let err = create_panel_context_for_text(" a ").unwrap_err();

    assert_eq!(err.code, "selection_text_too_short");
}
