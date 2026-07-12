pub mod ai;
pub mod app_lifecycle;
pub mod app_state;
pub mod commands;
pub mod config;
pub mod floating_window;
pub mod input_monitor;
pub mod platform;
pub mod security;
pub mod selection;
pub mod types;

use app_state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::load_or_default())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![app_lifecycle::AUTOSTART_ARG]),
        ))
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::save_app_behavior_config,
            commands::config::confirm_main_window_close,
            commands::config::save_provider_config,
            commands::config::set_default_provider,
            commands::config::delete_provider,
            commands::panel::show_floating_button,
            commands::panel::hide_floating_button,
            commands::panel::show_replacement_preset_panel,
            commands::panel::set_replacement_preset_panel_expanded,
            commands::panel::focus_floating_button,
            commands::panel::hide_replacement_preset_panel,
            commands::panel::show_ai_panel,
            commands::panel::hide_ai_panel,
            commands::panel::show_source_text_window,
            commands::panel::get_latest_source_text_context,
            commands::panel::hide_source_text_window,
            commands::panel::show_translate_result,
            commands::panel::hide_translate_result,
            commands::screenshot::show_screenshot_overlay,
            commands::screenshot::cancel_screenshot_translate,
            commands::screenshot::run_screenshot_translate,
            commands::platform::get_platform_capabilities,
            commands::selection::open_panel_for_text,
            commands::selection::get_latest_panel_context,
            commands::selection::open_panel_for_current_selection,
            commands::selection::copy_to_clipboard,
            commands::selection::replace_selected_text,
            commands::ai::run_ai_action,
            commands::ai::run_ai_follow_up,
            commands::ai::list_provider_models,
            commands::ai::test_provider_connection,
        ])
        .setup(|app| {
            if let Some(state) = app.try_state::<AppState>() {
                commands::config::refresh_launch_at_startup_registration(app.handle(), &state);
            }
            app_lifecycle::setup_background_lifecycle(app)?;
            app_lifecycle::apply_startup_visibility(app)?;
            input_monitor::start_background_monitor(app.handle().clone());
            Ok(())
        })
        .on_window_event(app_lifecycle::handle_window_event)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
