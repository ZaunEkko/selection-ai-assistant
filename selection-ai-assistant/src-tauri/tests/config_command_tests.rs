use selection_ai_assistant_lib::app_state::AppState;
use selection_ai_assistant_lib::commands::config::{
    get_config_from_state, get_runtime_preferences_from_state, get_settings_config_from_state,
    save_app_behavior_config_in_state, save_provider_config_in_state,
    save_provider_update_in_state,
};
use selection_ai_assistant_lib::commands::selection::validate_replacement_text;
use selection_ai_assistant_lib::config::{
    AiProviderConfig, AiProviderKind, AppBehaviorConfig, AppConfig, CloseButtonBehavior,
    ProviderUpdate, ReplacementTargetLanguage, SecretUpdate,
};

fn provider(id: &str, base_url: &str, model: &str) -> AiProviderConfig {
    AiProviderConfig {
        id: id.to_string(),
        name: "Test Provider".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        provider_kind: AiProviderKind::OpenAiCompatible,
        api_key: "test-api-key".to_string(),
        api_key_ref: format!("credential://{id}"),
        headers: Vec::new(),
    }
}

fn provider_update(
    original_provider_id: Option<&str>,
    id: &str,
    api_key: SecretUpdate,
) -> ProviderUpdate {
    ProviderUpdate {
        original_provider_id: original_provider_id.map(ToString::to_string),
        id: id.to_string(),
        name: "Updated Provider".to_string(),
        base_url: "https://example.com/v1".to_string(),
        model: "updated-model".to_string(),
        provider_kind: AiProviderKind::OpenAiCompatible,
        api_key,
        api_key_ref: format!("credential://{id}"),
    }
}

#[test]
fn settings_and_runtime_views_do_not_serialize_provider_secrets() {
    let mut saved_provider = provider("openai", "https://api.openai.com/v1", "gpt-test");
    saved_provider.api_key = "saved-api-key-secret".to_string();
    saved_provider.headers = vec![
        (
            "X-Trace-Context".to_string(),
            "trace-header-secret".to_string(),
        ),
        ("X-Tenant".to_string(), "tenant-header-secret".to_string()),
    ];
    let config = AppConfig {
        default_provider_id: Some("openai".to_string()),
        providers: vec![saved_provider],
        ..AppConfig::default()
    };
    let state = AppState::new(config);

    let settings = get_settings_config_from_state(&state).expect("settings view should load");
    let runtime =
        get_runtime_preferences_from_state(&state).expect("runtime preferences should load");
    let settings_json = serde_json::to_string(&settings).expect("settings view should serialize");
    let runtime_json = serde_json::to_string(&runtime).expect("runtime view should serialize");

    assert!(settings.providers[0].api_key_configured);
    assert!(settings.providers[0].custom_headers_configured);
    for serialized in [&settings_json, &runtime_json] {
        assert!(!serialized.contains("saved-api-key-secret"));
        assert!(!serialized.contains("trace-header-secret"));
        assert!(!serialized.contains("tenant-header-secret"));
        assert!(!serialized.contains("\"apiKey\":"));
        assert!(!serialized.contains("\"headers\":"));
    }
}

#[test]
fn provider_update_resolves_secret_keep_replace_and_clear() {
    let mut saved_provider = provider("openai", "https://api.openai.com/v1", "gpt-test");
    saved_provider.api_key = "saved-api-key".to_string();
    let config = AppConfig {
        providers: vec![saved_provider],
        ..AppConfig::default()
    };

    let kept = provider_update(Some("openai"), "openai", SecretUpdate::Keep).resolve(&config);
    let replaced = provider_update(
        Some("openai"),
        "openai",
        SecretUpdate::Replace {
            value: "replacement-api-key".to_string(),
        },
    )
    .resolve(&config);
    let cleared = provider_update(Some("openai"), "openai", SecretUpdate::Clear).resolve(&config);

    assert_eq!(kept.api_key, "saved-api-key");
    assert_eq!(replaced.api_key, "replacement-api-key");
    assert_eq!(cleared.api_key, "");
}

#[test]
fn provider_update_preserves_existing_custom_headers() {
    let mut saved_provider = provider("openai", "https://api.openai.com/v1", "gpt-test");
    saved_provider.headers = vec![
        (
            "X-Trace-Context".to_string(),
            "trace-header-value".to_string(),
        ),
        ("X-Tenant".to_string(), "tenant-header-value".to_string()),
    ];
    let expected_headers = saved_provider.headers.clone();
    let config = AppConfig {
        providers: vec![saved_provider],
        ..AppConfig::default()
    };

    let resolved = provider_update(
        Some("openai"),
        "openai",
        SecretUpdate::Replace {
            value: "new-api-key".to_string(),
        },
    )
    .resolve(&config);

    assert_eq!(resolved.headers, expected_headers);
}

#[test]
fn save_provider_update_uses_original_provider_id_when_renaming() {
    let mut saved_provider = provider("old-id", "https://old.example/v1", "old-model");
    saved_provider.api_key = "saved-api-key".to_string();
    saved_provider.headers = vec![(
        "X-Trace-Context".to_string(),
        "saved-header-value".to_string(),
    )];
    let state = AppState::new(AppConfig {
        default_provider_id: Some("old-id".to_string()),
        providers: vec![saved_provider],
        ..AppConfig::default()
    });

    let config = save_provider_update_in_state(
        &state,
        provider_update(Some("old-id"), "new-id", SecretUpdate::Keep),
    )
    .expect("renamed provider should save");

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].id, "new-id");
    assert_eq!(config.providers[0].api_key, "saved-api-key");
    assert_eq!(
        config.providers[0].headers,
        vec![(
            "X-Trace-Context".to_string(),
            "saved-header-value".to_string()
        )]
    );
    assert_eq!(config.default_provider_id.as_deref(), Some("new-id"));
}

#[test]
fn validate_replacement_text_rejects_empty_text() {
    let err = validate_replacement_text("   ").expect_err("empty replacement should fail");
    assert_eq!(err.code, "replacement_text_required");
}

#[test]
fn save_app_behavior_config_updates_startup_and_close_preferences() {
    let state = AppState::new(AppConfig::default());

    let config = save_app_behavior_config_in_state(
        &state,
        AppBehaviorConfig {
            hotkey: "Ctrl+Alt+T".to_string(),
            launch_at_startup: true,
            start_minimized_to_tray: true,
            close_button_behavior: CloseButtonBehavior::ExitApp,
            replacement_target_language: ReplacementTargetLanguage::Korean,
            replacement_custom_target: "韩语敬语".to_string(),
            translation_target_language: ReplacementTargetLanguage::MorseCode,
            translation_custom_target: "甲骨文风格".to_string(),
        },
    )
    .expect("app behavior config should save");

    assert_eq!(config.hotkey, "Ctrl+Alt+T");
    assert!(config.launch_at_startup);
    assert!(config.start_minimized_to_tray);
    assert_eq!(config.close_button_behavior, CloseButtonBehavior::ExitApp);
    assert_eq!(
        config.replacement_target_language,
        ReplacementTargetLanguage::Korean
    );
    assert_eq!(config.replacement_custom_target, "韩语敬语");
    assert_eq!(
        config.translation_target_language,
        ReplacementTargetLanguage::MorseCode
    );
    assert_eq!(config.translation_custom_target, "甲骨文风格");

    let stored = get_config_from_state(&state).expect("config should be readable");
    assert_eq!(stored.hotkey, "Ctrl+Alt+T");
    assert!(stored.launch_at_startup);
    assert!(stored.start_minimized_to_tray);
    assert_eq!(stored.close_button_behavior, CloseButtonBehavior::ExitApp);
    assert_eq!(
        stored.replacement_target_language,
        ReplacementTargetLanguage::Korean
    );
    assert_eq!(stored.replacement_custom_target, "韩语敬语");
    assert_eq!(
        stored.translation_target_language,
        ReplacementTargetLanguage::MorseCode
    );
    assert_eq!(stored.translation_custom_target, "甲骨文风格");
}

#[test]
fn save_app_behavior_config_persists_to_settings_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("settings.json");
    let state = AppState::new_with_settings_path(AppConfig::default(), path.clone());

    save_app_behavior_config_in_state(
        &state,
        AppBehaviorConfig {
            hotkey: "Ctrl+Alt+K".to_string(),
            launch_at_startup: true,
            start_minimized_to_tray: true,
            close_button_behavior: CloseButtonBehavior::MinimizeToTray,
            replacement_target_language: ReplacementTargetLanguage::Custom,
            replacement_custom_target: "日文自然口语".to_string(),
            translation_target_language: ReplacementTargetLanguage::Custom,
            translation_custom_target: "象形文字".to_string(),
        },
    )
    .expect("app behavior config should save");

    let loaded = AppConfig::load_from_path(&path).expect("settings should load from disk");
    assert_eq!(loaded.hotkey, "Ctrl+Alt+K");
    assert!(loaded.launch_at_startup);
    assert!(loaded.start_minimized_to_tray);
    assert_eq!(
        loaded.close_button_behavior,
        CloseButtonBehavior::MinimizeToTray
    );
    assert_eq!(
        loaded.replacement_target_language,
        ReplacementTargetLanguage::Custom
    );
    assert_eq!(loaded.replacement_custom_target, "日文自然口语");
    assert_eq!(
        loaded.translation_target_language,
        ReplacementTargetLanguage::Custom
    );
    assert_eq!(loaded.translation_custom_target, "象形文字");
}

#[test]
fn save_provider_config_adds_provider_and_sets_default() {
    let state = AppState::new(AppConfig::default());

    let config = save_provider_config_in_state(
        &state,
        provider("openai", "https://api.openai.com/v1", "gpt-test"),
    )
    .expect("provider should save");

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].id, "openai");
    assert_eq!(config.default_provider_id.as_deref(), Some("openai"));
}

#[test]
fn save_provider_config_updates_existing_provider() {
    let state = AppState::new(AppConfig::default());
    save_provider_config_in_state(
        &state,
        provider("openai", "https://api.openai.com/v1", "gpt-test"),
    )
    .expect("provider should save");

    let config = save_provider_config_in_state(
        &state,
        provider("openai", "https://example.com/v1", "gpt-next"),
    )
    .expect("provider should update");

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].base_url, "https://example.com/v1");
    assert_eq!(config.providers[0].model, "gpt-next");
}

#[test]
fn save_provider_config_updates_default_provider_to_saved_provider() {
    let state = AppState::new(AppConfig::default());
    save_provider_config_in_state(
        &state,
        provider("openai", "https://api.openai.com/v1", "gpt-test"),
    )
    .expect("first provider should save");

    let config = save_provider_config_in_state(
        &state,
        provider("openrouter", "https://openrouter.ai/api/v1", "gpt-next"),
    )
    .expect("second provider should save");

    assert_eq!(config.default_provider_id.as_deref(), Some("openrouter"));
}

#[test]
fn save_provider_config_rejects_missing_required_fields() {
    let state = AppState::new(AppConfig::default());

    let err = save_provider_config_in_state(
        &state,
        provider("", "https://api.openai.com/v1", "gpt-test"),
    )
    .expect_err("missing id should fail");
    assert_eq!(err.code, "provider_id_required");

    let err = save_provider_config_in_state(&state, provider("openai", "", "gpt-test"))
        .expect_err("missing base URL should fail");
    assert_eq!(err.code, "provider_config_incomplete");
}

#[test]
fn get_config_from_state_returns_current_config() {
    let state = AppState::new(AppConfig::default());
    save_provider_config_in_state(
        &state,
        provider("openai", "https://api.openai.com/v1", "gpt-test"),
    )
    .expect("provider should save");

    let config = get_config_from_state(&state).expect("config should load");

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].id, "openai");
}

#[test]
fn save_provider_config_allows_empty_model_for_model_loading_flow() {
    let state = AppState::new(AppConfig::default());

    let config =
        save_provider_config_in_state(&state, provider("openai", "https://api.openai.com/v1", ""))
            .expect("provider without model should save");

    assert_eq!(config.providers[0].model, "");
}

#[test]
fn save_provider_config_persists_provider_to_settings_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("settings.json");
    let state = AppState::new_with_settings_path(AppConfig::default(), path.clone());

    save_provider_config_in_state(
        &state,
        provider("openai", "https://api.openai.com/v1", "gpt-test"),
    )
    .expect("provider should save");

    let loaded = AppConfig::load_from_path(&path).expect("settings should load from disk");
    assert_eq!(loaded.default_provider_id.as_deref(), Some("openai"));
    assert_eq!(loaded.providers[0].api_key, "test-api-key");
}

#[test]
fn save_provider_config_does_not_mutate_memory_when_settings_write_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let blocking_file = dir.path().join("not-a-dir");
    std::fs::write(&blocking_file, "blocks settings directory").expect("blocking file");
    let path = blocking_file.join("settings.json");
    let state = AppState::new_with_settings_path(AppConfig::default(), path);

    let err = save_provider_config_in_state(
        &state,
        provider("openai", "https://api.openai.com/v1", "gpt-test"),
    )
    .expect_err("settings write should fail");

    assert_eq!(err.code, "config_save_failed");
    let config = get_config_from_state(&state).expect("config should still be readable");
    assert!(config.providers.is_empty());
    assert_eq!(config.default_provider_id, None);
}

#[test]
fn app_state_preserves_settings_path_after_corrupt_settings_then_saves_same_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("nested").join("settings.json");
    std::fs::create_dir_all(path.parent().expect("settings parent")).expect("settings parent");
    std::fs::write(&path, "{not valid json").expect("corrupt settings");

    let state = AppState::load_or_default_from_path(path.clone())
        .expect("state should fall back to default config");
    let config = get_config_from_state(&state).expect("fallback config should be readable");
    assert!(config.providers.is_empty());

    save_provider_config_in_state(
        &state,
        provider("openai", "https://api.openai.com/v1", "gpt-test"),
    )
    .expect("provider should save to preserved settings path");

    let loaded = AppConfig::load_from_path(&path).expect("settings should load from original path");
    assert_eq!(loaded.default_provider_id.as_deref(), Some("openai"));
    assert_eq!(loaded.providers[0].id, "openai");
}

#[test]
fn app_state_loads_existing_settings_from_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("nested").join("settings.json");
    let saved = AppConfig {
        default_provider_id: Some("openai".to_string()),
        providers: vec![provider("openai", "https://api.openai.com/v1", "gpt-test")],
        ..AppConfig::default()
    };
    saved.save_to_path(&path).expect("settings should save");

    let state = AppState::load_or_default_from_path(path).expect("state should load");
    let config = get_config_from_state(&state).expect("config should be readable");

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].id, "openai");
}
