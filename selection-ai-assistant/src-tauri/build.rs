const COMMANDS: &[&str] = &[
    "get_config",
    "get_runtime_preferences",
    "save_app_behavior_config",
    "save_output_target_preferences",
    "confirm_main_window_close",
    "save_provider_config",
    "set_default_provider",
    "delete_provider",
    "show_floating_button",
    "hide_floating_button",
    "show_replacement_preset_panel",
    "set_replacement_preset_panel_expanded",
    "focus_floating_button",
    "hide_replacement_preset_panel",
    "show_ai_panel",
    "hide_ai_panel",
    "show_source_text_window",
    "get_latest_source_text_context",
    "hide_source_text_window",
    "show_translate_result",
    "hide_translate_result",
    "show_screenshot_overlay",
    "cancel_screenshot_translate",
    "run_screenshot_translate",
    "get_platform_capabilities",
    "open_panel_for_text",
    "get_latest_panel_context",
    "open_panel_for_current_selection",
    "copy_to_clipboard",
    "replace_selected_text",
    "run_ai_action",
    "run_ai_follow_up",
    "list_provider_models",
    "test_provider_connection",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to build Tauri application");
}
