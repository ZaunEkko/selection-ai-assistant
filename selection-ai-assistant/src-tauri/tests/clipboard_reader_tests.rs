use selection_ai_assistant_lib::selection::{
    clipboard_reader::{
        clipboard_restore_attempt_sequence, empty_clipboard_outcome,
        should_accept_selected_text_after_restore,
        should_block_clipboard_fallback_after_uia_result,
        should_prepare_conservative_clipboard_capture, should_use_clipboard_fallback,
        ClipboardFallbackContext, ClipboardFormatSnapshot, ClipboardRestorePlan,
    },
    uia_reader::{SelectionConfidence, UiaSelectionResult},
};

#[test]
fn blocks_clipboard_for_disabled_apps() {
    let context = ClipboardFallbackContext {
        clipboard_fallback_enabled: true,
        process_name: "Bitwarden.exe".to_string(),
        disabled_apps: vec!["Bitwarden.exe".to_string()],
        is_password_control: false,
        is_elevated_window: false,
        disable_in_elevated_windows: true,
    };

    assert!(!should_use_clipboard_fallback(&context));
}

#[test]
fn blocks_clipboard_for_password_control() {
    let context = ClipboardFallbackContext {
        clipboard_fallback_enabled: true,
        process_name: "chrome.exe".to_string(),
        disabled_apps: Vec::new(),
        is_password_control: true,
        is_elevated_window: false,
        disable_in_elevated_windows: true,
    };

    assert!(!should_use_clipboard_fallback(&context));
}

#[test]
fn allows_clipboard_for_normal_window() {
    let context = ClipboardFallbackContext {
        clipboard_fallback_enabled: true,
        process_name: "chrome.exe".to_string(),
        disabled_apps: Vec::new(),
        is_password_control: false,
        is_elevated_window: false,
        disable_in_elevated_windows: true,
    };

    assert!(should_use_clipboard_fallback(&context));
}

#[test]
fn blocks_clipboard_fallback_after_uia_password_result() {
    let uia_password_result = UiaSelectionResult {
        text: None,
        rects: Vec::new(),
        is_password_control: true,
        confidence: SelectionConfidence::Low,
    };
    let context = ClipboardFallbackContext {
        clipboard_fallback_enabled: true,
        process_name: "chrome.exe".to_string(),
        disabled_apps: Vec::new(),
        is_password_control: should_block_clipboard_fallback_after_uia_result(Some(
            &uia_password_result,
        )),
        is_elevated_window: false,
        disable_in_elevated_windows: true,
    };

    assert!(!should_use_clipboard_fallback(&context));
}

#[test]
fn allows_conservative_capture_for_empty_plain_or_mixed_text_clipboard() {
    assert!(should_prepare_conservative_clipboard_capture(0, false));
    assert!(should_prepare_conservative_clipboard_capture(1, true));
    assert!(should_prepare_conservative_clipboard_capture(2, true));
    assert!(should_prepare_conservative_clipboard_capture(6, true));
}

#[test]
fn blocks_conservative_capture_for_non_text_clipboard_formats() {
    assert!(!should_prepare_conservative_clipboard_capture(1, false));
    assert!(!should_prepare_conservative_clipboard_capture(3, false));
}

#[test]
fn restore_attempt_sequence_retries_original_then_cleans_up_with_empty_clipboard() {
    assert_eq!(
        clipboard_restore_attempt_sequence(ClipboardRestorePlan::Text("original".to_string()), 2),
        vec![
            ClipboardRestorePlan::Text("original".to_string()),
            ClipboardRestorePlan::Text("original".to_string()),
            ClipboardRestorePlan::Text("original".to_string()),
            ClipboardRestorePlan::Empty,
        ]
    );
}

#[test]
fn restore_attempt_sequence_retries_format_snapshots_then_cleans_up_with_empty_clipboard() {
    let snapshot = ClipboardRestorePlan::Formats(vec![ClipboardFormatSnapshot {
        format: 13,
        data: vec![b'o', 0, b'k', 0, 0, 0],
    }]);

    assert_eq!(
        clipboard_restore_attempt_sequence(snapshot.clone(), 2),
        vec![
            snapshot.clone(),
            snapshot.clone(),
            snapshot,
            ClipboardRestorePlan::Empty,
        ]
    );
}

#[test]
fn restore_attempt_sequence_retries_empty_clipboard_cleanup() {
    assert_eq!(
        clipboard_restore_attempt_sequence(ClipboardRestorePlan::Empty, 2),
        vec![
            ClipboardRestorePlan::Empty,
            ClipboardRestorePlan::Empty,
            ClipboardRestorePlan::Empty,
            ClipboardRestorePlan::Empty,
        ]
    );
}

#[test]
fn selected_text_is_only_accepted_when_clipboard_restore_succeeded() {
    assert_eq!(
        should_accept_selected_text_after_restore(Some("selected text"), true),
        Some("selected text".to_string())
    );
    assert_eq!(
        should_accept_selected_text_after_restore(Some("selected text"), false),
        None
    );
}

#[test]
fn empty_clipboard_outcome_carries_skip_reason() {
    let outcome = empty_clipboard_outcome("disabled app");

    assert_eq!(outcome.text, None);
    assert!(!outcome.restored_original_clipboard);
    assert_eq!(outcome.skipped_reason, Some("disabled app".to_string()));
}
