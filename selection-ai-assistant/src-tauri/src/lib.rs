pub mod ai;
pub mod app_lifecycle;
pub mod app_state;
pub mod commands;
pub mod config;
pub mod floating_window;
pub mod input_monitor;
pub mod security;
pub mod selection;
pub mod types;

use app_state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::load_or_default())
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::save_provider_config,
            commands::panel::show_floating_button,
            commands::panel::hide_floating_button,
            commands::panel::show_ai_panel,
            commands::panel::hide_ai_panel,
            commands::selection::open_panel_for_text,
            commands::selection::get_latest_panel_context,
            commands::selection::open_panel_for_current_selection,
            commands::ai::run_ai_action,
            commands::ai::list_provider_models,
            commands::ai::test_provider_connection,
        ])
        .setup(|app| {
            app_lifecycle::setup_background_lifecycle(app)?;
            input_monitor::start_background_monitor(app.handle().clone());
            Ok(())
        })
        .on_window_event(app_lifecycle::handle_window_event)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
