use selection_ai_assistant_lib::app_lifecycle::{
    close_request_action_for_window, should_show_main_window_on_startup, CloseRequestAction,
};
use selection_ai_assistant_lib::config::{AppConfig, CloseButtonBehavior};

#[test]
fn default_startup_shows_main_window() {
    assert!(should_show_main_window_on_startup(&AppConfig::default()));
}

#[test]
fn configured_background_start_keeps_main_window_hidden() {
    let config = AppConfig {
        start_minimized_to_tray: true,
        ..AppConfig::default()
    };

    assert!(!should_show_main_window_on_startup(&config));
}

#[test]
fn main_window_close_asks_before_deciding_by_default() {
    assert_eq!(
        close_request_action_for_window("main", CloseButtonBehavior::Ask),
        CloseRequestAction::AskUser
    );
}

#[test]
fn main_window_close_can_minimize_to_tray_without_prompt() {
    assert_eq!(
        close_request_action_for_window("main", CloseButtonBehavior::MinimizeToTray),
        CloseRequestAction::MinimizeToTray
    );
}

#[test]
fn main_window_close_can_exit_app_without_prompt() {
    assert_eq!(
        close_request_action_for_window("main", CloseButtonBehavior::ExitApp),
        CloseRequestAction::ExitApp
    );
}

#[test]
fn assistant_overlay_windows_do_not_use_background_close_behavior() {
    assert_eq!(
        close_request_action_for_window("floating-button", CloseButtonBehavior::MinimizeToTray),
        CloseRequestAction::Ignore
    );
    assert_eq!(
        close_request_action_for_window("ai-panel", CloseButtonBehavior::ExitApp),
        CloseRequestAction::Ignore
    );
    assert_eq!(
        close_request_action_for_window("other", CloseButtonBehavior::Ask),
        CloseRequestAction::Ignore
    );
}
