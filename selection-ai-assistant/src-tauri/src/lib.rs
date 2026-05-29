pub mod ai;
pub mod app_state;
pub mod commands;
pub mod config;
pub mod floating_window;
pub mod input_monitor;
pub mod security;
pub mod selection;
pub mod types;

use app_state::AppState;
use config::AppConfig;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new(AppConfig::default()))
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::save_provider_config,
            commands::panel::show_floating_button,
            commands::panel::hide_floating_button,
            commands::panel::show_ai_panel,
            commands::panel::hide_ai_panel,
            commands::selection::open_panel_for_text,
            commands::ai::run_ai_action,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
