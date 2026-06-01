use selection_ai_assistant_lib::app_lifecycle::should_hide_to_background_on_close;

#[test]
fn main_window_close_hides_to_background() {
    assert!(should_hide_to_background_on_close("main"));
}

#[test]
fn assistant_overlay_windows_do_not_use_background_close_behavior() {
    assert!(!should_hide_to_background_on_close("floating-button"));
    assert!(!should_hide_to_background_on_close("ai-panel"));
    assert!(!should_hide_to_background_on_close("other"));
}
