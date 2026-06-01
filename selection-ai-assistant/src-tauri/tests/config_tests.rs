use selection_ai_assistant_lib::config::AppConfig;

#[test]
fn settings_path_uses_local_app_config_directory_when_available() {
    let path = AppConfig::settings_path().expect("settings path should resolve");

    assert!(path.ends_with("selection-ai-assistant/settings.json"));

    if let Some(local_dir) = dirs::config_local_dir().or_else(dirs::data_local_dir) {
        assert!(
            path.starts_with(local_dir),
            "settings path should use local app data directory when available: {path:?}"
        );
    }
}

#[test]
fn save_to_path_replaces_existing_settings_and_removes_temp_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("settings.json");
    std::fs::write(&path, "old settings should be replaced").expect("old settings");

    let config = AppConfig {
        default_provider_id: Some("openai".to_string()),
        ..AppConfig::default()
    };

    config.save_to_path(&path).expect("settings should save");

    let loaded = AppConfig::load_from_path(&path).expect("settings should load");
    assert_eq!(loaded.default_provider_id.as_deref(), Some("openai"));

    let leftover_temp_files: Vec<_> = std::fs::read_dir(dir.path())
        .expect("settings dir should be readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftover_temp_files.is_empty());
}

#[test]
fn default_config_contains_privacy_defaults() {
    let config = AppConfig::default();

    assert_eq!(config.hover_radius, 90.0);
    assert_eq!(config.hover_delay_ms, 1_000);
    assert_eq!(config.candidate_timeout_ms, 4_000);
    assert!(config.clipboard_fallback_enabled);
    assert!(config.show_clipboard_privacy_warning_on_first_use);
    assert!(config.disable_in_elevated_windows);
    assert!(config.manual_hotkey_always_enabled);
    assert!(config.is_disabled_process("Bitwarden.exe"));
    assert!(config.is_disabled_process("bitwarden.exe"));
}

#[test]
fn settings_schema_defaults_missing_fields_for_backward_compatibility() {
    let config: AppConfig = serde_json::from_str(
        r#"{
            "defaultProviderId": "openai",
            "providers": [
                {
                    "id": "openai",
                    "name": "OpenAI",
                    "baseUrl": "https://api.openai.com/v1",
                    "model": "gpt-test"
                }
            ]
        }"#,
    )
    .expect("older settings schema should load with defaults");

    assert_eq!(config.providers[0].api_key, "");
    assert_eq!(config.providers[0].api_key_ref, "");
    assert!(config.providers[0].headers.is_empty());
    assert_eq!(config.hotkey, AppConfig::default().hotkey);
    assert!(config.clipboard_fallback_enabled);
    assert!(config.manual_hotkey_always_enabled);
}
