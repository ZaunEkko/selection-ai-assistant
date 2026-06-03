pub mod events;

pub fn start_background_monitor(app: tauri::AppHandle) {
    crate::platform::start_background_monitor(app);
}

pub fn notify_ai_panel_closed_by_user(assistant_rects: Vec<crate::types::Rect>) {
    crate::platform::notify_ai_panel_closed_by_user(assistant_rects);
}
