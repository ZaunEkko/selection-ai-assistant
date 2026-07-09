use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::app_lifecycle;
use crate::app_state::AppState;
use crate::config::{AiProviderConfig, AppBehaviorConfig, AppConfig, CloseButtonBehavior};
use crate::types::PublicError;

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<AppConfig, PublicError> {
    get_config_from_state(&state)
}

pub fn get_config_from_state(state: &AppState) -> Result<AppConfig, PublicError> {
    state
        .config
        .lock()
        .map(|config| config.clone())
        .map_err(|err| PublicError {
            code: "config_lock_failed".to_string(),
            message: err.to_string(),
        })
}

#[tauri::command]
pub fn save_app_behavior_config(
    app: AppHandle,
    state: State<'_, AppState>,
    preferences: AppBehaviorConfig,
) -> Result<AppConfig, PublicError> {
    sync_launch_at_startup(&app, preferences.launch_at_startup)?;
    save_app_behavior_config_in_state(&state, preferences)
}

fn sync_launch_at_startup(app: &AppHandle, enabled: bool) -> Result<(), PublicError> {
    let autostart = app.autolaunch();
    let result = if enabled {
        autostart.enable()
    } else {
        autostart.disable()
    };

    result.map_err(|err| PublicError {
        code: "autostart_update_failed".to_string(),
        message: format!("更新开机自启设置失败：{err}"),
    })
}

pub fn save_app_behavior_config_in_state(
    state: &AppState,
    preferences: AppBehaviorConfig,
) -> Result<AppConfig, PublicError> {
    let mut config = state.config.lock().map_err(|err| PublicError {
        code: "config_lock_failed".to_string(),
        message: err.to_string(),
    })?;

    let mut candidate = config.clone();
    candidate.hotkey = preferences.hotkey.trim().to_string();
    candidate.launch_at_startup = preferences.launch_at_startup;
    candidate.start_minimized_to_tray = preferences.start_minimized_to_tray;
    candidate.close_button_behavior = preferences.close_button_behavior;
    candidate.replacement_target_language = preferences.replacement_target_language;
    candidate.replacement_custom_target = preferences.replacement_custom_target.trim().to_string();
    candidate.translation_target_language = preferences.translation_target_language;
    candidate.translation_custom_target = preferences.translation_custom_target.trim().to_string();

    if let Some(path) = &state.settings_path {
        candidate.save_to_path(path).map_err(|err| PublicError {
            code: "config_save_failed".to_string(),
            message: format!("Failed to save settings: {err}"),
        })?;
    }

    *config = candidate.clone();
    Ok(candidate)
}

#[tauri::command]
pub fn confirm_main_window_close(
    app: AppHandle,
    state: State<'_, AppState>,
    behavior: CloseButtonBehavior,
) -> Result<AppConfig, PublicError> {
    let current = get_config_from_state(&state)?;
    let config = save_app_behavior_config_in_state(
        &state,
        AppBehaviorConfig {
            hotkey: current.hotkey,
            launch_at_startup: current.launch_at_startup,
            start_minimized_to_tray: current.start_minimized_to_tray,
            close_button_behavior: behavior,
            replacement_target_language: current.replacement_target_language,
            replacement_custom_target: current.replacement_custom_target,
            translation_target_language: current.translation_target_language,
            translation_custom_target: current.translation_custom_target,
        },
    )?;

    app_lifecycle::apply_main_close_choice(&app, behavior).map_err(|err| PublicError {
        code: "main_close_choice_failed".to_string(),
        message: err.to_string(),
    })?;

    Ok(config)
}

#[tauri::command]
pub fn save_provider_config(
    state: State<'_, AppState>,
    provider: AiProviderConfig,
) -> Result<AppConfig, PublicError> {
    save_provider_config_in_state(&state, provider)
}

pub fn save_provider_config_in_state(
    state: &AppState,
    provider: AiProviderConfig,
) -> Result<AppConfig, PublicError> {
    if provider.id.trim().is_empty() {
        return Err(PublicError {
            code: "provider_id_required".to_string(),
            message: "Provider id is required.".to_string(),
        });
    }

    if provider.base_url.trim().is_empty() {
        return Err(PublicError {
            code: "provider_config_incomplete".to_string(),
            message: "Provider base URL is required.".to_string(),
        });
    }

    let mut config = state.config.lock().map_err(|err| PublicError {
        code: "config_lock_failed".to_string(),
        message: err.to_string(),
    })?;

    let mut candidate = config.clone();
    if let Some(existing) = candidate
        .providers
        .iter_mut()
        .find(|item| item.id == provider.id)
    {
        *existing = provider.clone();
    } else {
        candidate.providers.push(provider.clone());
    }

    candidate.default_provider_id = Some(provider.id);

    if let Some(path) = &state.settings_path {
        candidate.save_to_path(path).map_err(|err| PublicError {
            code: "config_save_failed".to_string(),
            message: format!("Failed to save settings: {err}"),
        })?;
    }

    *config = candidate.clone();
    Ok(candidate)
}

#[tauri::command]
pub fn set_default_provider(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<AppConfig, PublicError> {
    let mut config = state.config.lock().map_err(|err| PublicError {
        code: "config_lock_failed".to_string(),
        message: err.to_string(),
    })?;

    if !config.providers.iter().any(|p| p.id == provider_id) {
        return Err(PublicError {
            code: "provider_not_found".to_string(),
            message: format!("Provider with id '{}' not found.", provider_id),
        });
    }

    let mut candidate = config.clone();
    candidate.default_provider_id = Some(provider_id);

    if let Some(path) = &state.settings_path {
        candidate.save_to_path(path).map_err(|err| PublicError {
            code: "config_save_failed".to_string(),
            message: format!("Failed to save settings: {err}"),
        })?;
    }

    *config = candidate.clone();
    Ok(candidate)
}

#[tauri::command]
pub fn delete_provider(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<AppConfig, PublicError> {
    let mut config = state.config.lock().map_err(|err| PublicError {
        code: "config_lock_failed".to_string(),
        message: err.to_string(),
    })?;

    let mut candidate = config.clone();
    let original_len = candidate.providers.len();
    candidate.providers.retain(|p| p.id != provider_id);

    if candidate.providers.len() == original_len {
        return Err(PublicError {
            code: "provider_not_found".to_string(),
            message: format!("Provider with id '{}' not found.", provider_id),
        });
    }

    if candidate.default_provider_id.as_ref() == Some(&provider_id) {
        candidate.default_provider_id = candidate.providers.first().map(|p| p.id.clone());
    }

    if let Some(path) = &state.settings_path {
        candidate.save_to_path(path).map_err(|err| PublicError {
            code: "config_save_failed".to_string(),
            message: format!("Failed to save settings: {err}"),
        })?;
    }

    *config = candidate.clone();
    Ok(candidate)
}
