use selection_ai_assistant_lib::selection::clipboard_reader::{
    empty_clipboard_outcome, should_use_clipboard_fallback, ClipboardFallbackContext,
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
fn empty_clipboard_outcome_carries_skip_reason() {
    let outcome = empty_clipboard_outcome("disabled app");

    assert_eq!(outcome.text, None);
    assert!(!outcome.restored_original_clipboard);
    assert_eq!(outcome.skipped_reason, Some("disabled app".to_string()));
}
