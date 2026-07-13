use tauri::WebviewWindow;

use crate::types::PublicError;

pub fn require_webview_label(
    webview: &WebviewWindow,
    allowed_labels: &[&str],
) -> Result<(), PublicError> {
    require_label(webview.label(), allowed_labels)
}

pub fn require_label(label: &str, allowed_labels: &[&str]) -> Result<(), PublicError> {
    if allowed_labels.contains(&label) {
        Ok(())
    } else {
        Err(PublicError {
            code: "command_window_unauthorized".to_string(),
            message: format!("窗口 `{label}` 无权执行此操作。"),
        })
    }
}
