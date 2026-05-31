use tauri::State;

use crate::app_state::AppState;
use crate::config::{AiProviderConfig, AppConfig};
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
