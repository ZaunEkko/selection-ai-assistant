use selection_ai_assistant_lib::config::{
    AiProviderKind, AppConfig, CloseButtonBehavior, ReplacementTargetLanguage,
};

#[test]
fn replacement_preset_window_shows_without_stealing_focus_on_windows() {
    let panel_rs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("commands")
        .join("panel.rs");
    let panel_rs = std::fs::read_to_string(panel_rs_path).expect("panel.rs should load");

    assert!(
        panel_rs.contains("SW_SHOWNOACTIVATE")
            && panel_rs.contains("SW_HIDE")
            && panel_rs.contains("show_replacement_preset_without_activation(&window)")
            && panel_rs.contains("hide_replacement_preset_window(&window)")
            && panel_rs.contains("target_preset_panel_hidden")
            && panel_rs.contains("if !floating.is_visible().unwrap_or(false)"),
        "the target preset window must not steal focus or survive after the mini action bar is hidden"
    );
}

#[test]
fn plain_release_build_defaults_to_embedded_custom_protocol() {
    let cargo_toml_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let cargo_toml = std::fs::read_to_string(cargo_toml_path).expect("Cargo.toml should load");

    assert!(
        cargo_toml.contains("default = [\"custom-protocol\"]")
            && cargo_toml.contains("custom-protocol = [\"tauri/custom-protocol\"]"),
        "plain release builds should embed frontend assets instead of loading the localhost dev URL"
    );
}

#[test]
fn all_windows_binaries_use_gui_subsystem_to_avoid_console_window() {
    let main_rs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("main.rs");
    let main_rs = std::fs::read_to_string(main_rs_path).expect("main.rs should load");

    assert!(
        main_rs.contains("cfg_attr(target_os = \"windows\", windows_subsystem = \"windows\")"),
        "Windows debug and release builds should not open an extra console window next to the app"
    );
}

#[test]
fn overlay_windows_have_tauri_capability_permissions() {
    let capabilities_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("capabilities")
        .join("default.json");
    let capabilities: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(capabilities_path).expect("capabilities file should load"),
    )
    .expect("capabilities file should be valid json");
    let windows = capabilities["windows"]
        .as_array()
        .expect("capability windows should be an array");

    for label in ["source-text", "translate-result", "screenshot-overlay"] {
        assert!(
            windows.iter().any(|window| window.as_str() == Some(label)),
            "{label} window needs capability access to listen for events and invoke window commands"
        );
    }
}

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
    assert!(!config.launch_at_startup);
    assert!(!config.start_minimized_to_tray);
    assert_eq!(config.close_button_behavior, CloseButtonBehavior::Ask);
    assert_eq!(
        config.replacement_target_language,
        ReplacementTargetLanguage::Auto
    );
    assert_eq!(config.replacement_custom_target, "");
    assert_eq!(
        config.translation_target_language,
        ReplacementTargetLanguage::Auto
    );
    assert_eq!(config.translation_custom_target, "");
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

    assert_eq!(
        config.providers[0].provider_kind,
        AiProviderKind::OpenAiCompatible
    );
    assert_eq!(config.providers[0].api_key, "");
    assert_eq!(config.providers[0].api_key_ref, "");
    assert!(config.providers[0].headers.is_empty());
    assert_eq!(config.hotkey, AppConfig::default().hotkey);
    assert!(config.clipboard_fallback_enabled);
    assert!(config.manual_hotkey_always_enabled);
    assert!(!config.launch_at_startup);
    assert!(!config.start_minimized_to_tray);
    assert_eq!(config.close_button_behavior, CloseButtonBehavior::Ask);
    assert_eq!(
        config.replacement_target_language,
        ReplacementTargetLanguage::Auto
    );
    assert_eq!(config.replacement_custom_target, "");
    assert_eq!(
        config.translation_target_language,
        ReplacementTargetLanguage::Auto
    );
    assert_eq!(config.translation_custom_target, "");
}
