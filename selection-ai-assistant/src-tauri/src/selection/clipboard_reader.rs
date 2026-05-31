#[derive(Debug, Clone)]
pub struct ClipboardFallbackContext {
    pub clipboard_fallback_enabled: bool,
    pub process_name: String,
    pub disabled_apps: Vec<String>,
    pub is_password_control: bool,
    pub is_elevated_window: bool,
    pub disable_in_elevated_windows: bool,
}

pub fn should_use_clipboard_fallback(context: &ClipboardFallbackContext) -> bool {
    if !context.clipboard_fallback_enabled {
        return false;
    }

    if context.is_password_control {
        return false;
    }

    if context.disable_in_elevated_windows && context.is_elevated_window {
        return false;
    }

    let disabled = context
        .disabled_apps
        .iter()
        .any(|app| app.eq_ignore_ascii_case(&context.process_name));

    !disabled
}

pub fn should_prepare_conservative_clipboard_capture(
    format_count: u32,
    unicode_text_available: bool,
) -> bool {
    format_count == 0 || (format_count == 1 && unicode_text_available)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardRestorePlan {
    Text(String),
    Empty,
}

pub fn clipboard_restore_attempt_sequence(
    plan: ClipboardRestorePlan,
    retry_count: usize,
) -> Vec<ClipboardRestorePlan> {
    let mut attempts = vec![plan; retry_count.saturating_add(1)];
    attempts.push(ClipboardRestorePlan::Empty);
    attempts
}

pub fn should_accept_selected_text_after_restore(
    selected_text: Option<&str>,
    restored_clipboard: bool,
) -> Option<String> {
    if !restored_clipboard {
        return None;
    }

    selected_text
        .map(str::to_string)
        .filter(|text| text.chars().count() >= 2)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardReadOutcome {
    pub text: Option<String>,
    pub restored_original_clipboard: bool,
    pub skipped_reason: Option<String>,
}

pub fn empty_clipboard_outcome(reason: impl Into<String>) -> ClipboardReadOutcome {
    ClipboardReadOutcome {
        text: None,
        restored_original_clipboard: false,
        skipped_reason: Some(reason.into()),
    }
}
