use selection_ai_assistant_lib::config::AppConfig;

#[test]
fn default_config_contains_privacy_defaults() {
    let config = AppConfig::default();

    assert_eq!(config.hover_radius, 90.0);
    assert_eq!(config.hover_delay_ms, 220);
    assert_eq!(config.candidate_timeout_ms, 4_000);
    assert!(config.clipboard_fallback_enabled);
    assert!(config.show_clipboard_privacy_warning_on_first_use);
    assert!(config.disable_in_elevated_windows);
    assert!(config.manual_hotkey_always_enabled);
    assert!(config.is_disabled_process("Bitwarden.exe"));
    assert!(config.is_disabled_process("bitwarden.exe"));
}
