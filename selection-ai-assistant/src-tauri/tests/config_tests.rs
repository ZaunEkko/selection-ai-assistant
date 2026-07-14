use std::collections::{BTreeMap, BTreeSet};

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
fn app_manifest_commands_match_invoke_handler_commands() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let build_rs =
        std::fs::read_to_string(crate_root.join("build.rs")).expect("build.rs should load");
    let lib_rs =
        std::fs::read_to_string(crate_root.join("src").join("lib.rs")).expect("lib.rs should load");

    let manifest_commands: BTreeSet<_> = build_rs
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let command = line.strip_prefix('"')?.split('"').next()?;
            Some(command.to_string())
        })
        .collect();
    let handler_commands: BTreeSet<_> = lib_rs
        .split("tauri::generate_handler![")
        .nth(1)
        .expect("invoke handler should exist")
        .split("])")
        .next()
        .expect("invoke handler should close")
        .lines()
        .filter_map(|line| {
            let line = line.trim().trim_end_matches(',');
            (!line.is_empty()).then(|| line.rsplit("::").next().unwrap().to_string())
        })
        .collect();

    assert_eq!(manifest_commands, handler_commands);
}

#[test]
fn each_window_has_an_exact_minimum_capability() {
    let expected: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::from([
        (
            "main",
            BTreeSet::from([
                "allow-get-config",
                "allow-get-platform-capabilities",
                "allow-save-provider-config",
                "allow-set-default-provider",
                "allow-delete-provider",
                "allow-save-app-behavior-config",
                "allow-confirm-main-window-close",
                "allow-list-provider-models",
                "allow-test-provider-connection",
                "core:event:allow-listen",
                "core:event:allow-unlisten",
            ]),
        ),
        (
            "floating-button",
            BTreeSet::from([
                "allow-get-runtime-preferences",
                "allow-get-latest-panel-context",
                "allow-run-ai-action",
                "allow-replace-selected-text",
                "allow-open-panel-for-current-selection",
                "allow-show-replacement-preset-panel",
                "allow-hide-replacement-preset-panel",
                "allow-show-translate-result",
                "core:event:allow-listen",
                "core:event:allow-unlisten",
                "core:event:allow-emit",
            ]),
        ),
        (
            "replacement-preset",
            BTreeSet::from([
                "allow-get-runtime-preferences",
                "allow-save-output-target-preferences",
                "allow-set-replacement-preset-panel-expanded",
                "allow-focus-floating-button",
                "core:event:allow-listen",
                "core:event:allow-unlisten",
            ]),
        ),
        (
            "ai-panel",
            BTreeSet::from([
                "allow-get-latest-panel-context",
                "allow-run-ai-action",
                "allow-run-ai-follow-up",
                "allow-show-source-text-window",
                "allow-hide-source-text-window",
                "allow-hide-ai-panel",
                "core:event:allow-listen",
                "core:event:allow-unlisten",
                "core:window:allow-start-dragging",
            ]),
        ),
        (
            "source-text",
            BTreeSet::from([
                "allow-get-latest-source-text-context",
                "allow-hide-source-text-window",
                "core:event:allow-listen",
                "core:event:allow-unlisten",
                "core:window:allow-start-dragging",
            ]),
        ),
        (
            "translate-result",
            BTreeSet::from([
                "allow-hide-translate-result",
                "core:event:allow-listen",
                "core:event:allow-unlisten",
                "core:window:allow-start-dragging",
                "core:window:allow-start-resize-dragging",
            ]),
        ),
        (
            "screenshot-overlay",
            BTreeSet::from([
                "allow-get-runtime-preferences",
                "allow-cancel-screenshot-translate",
                "allow-run-screenshot-translate",
            ]),
        ),
    ]);
    let forbidden = [
        "core:default",
        "core:event:default",
        "core:window:default",
        "opener:default",
    ];
    let capabilities_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
    let mut actual = BTreeMap::<String, BTreeSet<String>>::new();

    for entry in std::fs::read_dir(capabilities_dir).expect("capabilities directory should load") {
        let entry = entry.expect("capability entry should load");
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let capability: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(entry.path()).expect("capability file should load"),
        )
        .expect("capability file should be valid json");
        let windows = capability["windows"]
            .as_array()
            .expect("capability windows should be an array");
        assert_eq!(
            windows.len(),
            1,
            "each capability must be bound to exactly one window"
        );
        let window = windows[0]
            .as_str()
            .expect("capability window should be a string")
            .to_string();
        let permissions: BTreeSet<String> = capability["permissions"]
            .as_array()
            .expect("capability permissions should be an array")
            .iter()
            .map(|permission| {
                permission
                    .as_str()
                    .expect("permission should be a string")
                    .to_string()
            })
            .collect();
        for permission in forbidden {
            assert!(
                !permissions.contains(permission),
                "{window} must not use broad permission {permission}"
            );
        }
        assert!(
            actual.insert(window.clone(), permissions).is_none(),
            "window {window} must have exactly one capability"
        );
    }

    let expected: BTreeMap<String, BTreeSet<String>> = expected
        .into_iter()
        .map(|(window, permissions)| {
            (
                window.to_string(),
                permissions.into_iter().map(str::to_string).collect(),
            )
        })
        .collect();
    assert_eq!(actual, expected);
    assert!(
        !actual["replacement-preset"].contains("allow-save-app-behavior-config"),
        "the preset window must not receive full app behavior write access"
    );
}

#[test]
fn production_and_development_csp_are_explicitly_scoped() {
    let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(config_path).expect("tauri config should load"),
    )
    .expect("tauri config should be valid json");
    let security = &config["app"]["security"];
    let production = security["csp"]
        .as_str()
        .expect("production CSP should be a string");
    let development = security["devCsp"]
        .as_str()
        .expect("development CSP should be a string");

    assert_eq!(
        production,
        "default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'"
    );
    assert_eq!(
        development,
        "default-src 'self'; connect-src ipc: http://ipc.localhost http://localhost:5173 ws://localhost:5173; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'"
    );
    assert!(!production.contains("localhost:5173"));
    assert!(!production.contains("https:"));
    assert!(!production.contains("ws:"));
    assert!(!development.contains("https:"));
    assert!(!development.contains("ws://127.0.0.1"));
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
