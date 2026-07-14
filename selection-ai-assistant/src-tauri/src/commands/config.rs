use std::path::Path;

use tauri::{AppHandle, State, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;

use crate::app_lifecycle;
use crate::app_state::AppState;
use crate::commands::access::require_webview_label;
use crate::commands::panel::TargetPresetKind;
use crate::config::{
    AiProviderConfig, AppBehaviorConfig, AppConfig, CloseButtonBehavior, ProviderUpdate,
    ReplacementTargetLanguage, RuntimePreferences, SettingsConfigView,
};
use crate::types::PublicError;

#[tauri::command]
pub fn get_config(
    webview: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<SettingsConfigView, PublicError> {
    require_webview_label(&webview, &["main"])?;
    get_settings_config_from_state(&state)
}

#[tauri::command]
pub fn get_runtime_preferences(
    webview: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<RuntimePreferences, PublicError> {
    require_webview_label(
        &webview,
        &[
            "floating-button",
            "replacement-preset",
            "screenshot-overlay",
        ],
    )?;
    get_runtime_preferences_from_state(&state)
}

pub fn get_config_from_state(state: &AppState) -> Result<AppConfig, PublicError> {
    state
        .config
        .lock()
        .map(|config| config.clone())
        .map_err(config_lock_error)
}

pub fn get_settings_config_from_state(state: &AppState) -> Result<SettingsConfigView, PublicError> {
    state
        .config
        .lock()
        .map(|config| SettingsConfigView::from(&*config))
        .map_err(config_lock_error)
}

pub fn get_runtime_preferences_from_state(
    state: &AppState,
) -> Result<RuntimePreferences, PublicError> {
    state
        .config
        .lock()
        .map(|config| RuntimePreferences::from(&*config))
        .map_err(config_lock_error)
}

#[tauri::command]
pub fn save_app_behavior_config(
    webview: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    preferences: AppBehaviorConfig,
) -> Result<RuntimePreferences, PublicError> {
    require_webview_label(&webview, &["main"])?;
    sync_launch_at_startup(&app, preferences.launch_at_startup)?;
    let config = save_app_behavior_config_in_state(&state, preferences)?;
    Ok(RuntimePreferences::from(&config))
}

pub(crate) fn sync_launch_at_startup(app: &AppHandle, enabled: bool) -> Result<(), PublicError> {
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

pub(crate) fn refresh_launch_at_startup_registration(app: &AppHandle, state: &AppState) {
    let enabled = state
        .config
        .lock()
        .map(|config| config.launch_at_startup)
        .unwrap_or(false);

    if enabled {
        if let Err(err) = sync_launch_at_startup(app, true) {
            eprintln!(
                "[autostart] failed to refresh startup registration: {}",
                err.message
            );
        }
    }
}

pub fn save_app_behavior_config_in_state(
    state: &AppState,
    preferences: AppBehaviorConfig,
) -> Result<AppConfig, PublicError> {
    let mut config = state.config.lock().map_err(config_lock_error)?;

    let mut candidate = config.clone();
    candidate.hotkey = preferences.hotkey.trim().to_string();
    candidate.launch_at_startup = preferences.launch_at_startup;
    candidate.start_minimized_to_tray = preferences.start_minimized_to_tray;
    candidate.close_button_behavior = preferences.close_button_behavior;
    candidate.replacement_target_language = preferences.replacement_target_language;
    candidate.replacement_custom_target = preferences.replacement_custom_target.trim().to_string();
    candidate.translation_target_language = preferences.translation_target_language;
    candidate.translation_custom_target = preferences.translation_custom_target.trim().to_string();

    persist_candidate(state.settings_path.as_deref(), &candidate)?;

    *config = candidate.clone();
    Ok(candidate)
}

#[tauri::command]
pub fn save_output_target_preferences(
    webview: WebviewWindow,
    state: State<'_, AppState>,
    kind: TargetPresetKind,
    target_language: ReplacementTargetLanguage,
    custom_target: String,
) -> Result<RuntimePreferences, PublicError> {
    require_webview_label(&webview, &["replacement-preset"])?;
    let config =
        save_output_target_preferences_in_state(&state, kind, target_language, custom_target)?;
    Ok(RuntimePreferences::from(&config))
}

pub fn save_output_target_preferences_in_state(
    state: &AppState,
    kind: TargetPresetKind,
    target_language: ReplacementTargetLanguage,
    custom_target: String,
) -> Result<AppConfig, PublicError> {
    let custom_target = custom_target.trim().to_string();
    if target_language == ReplacementTargetLanguage::Custom && custom_target.is_empty() {
        return Err(PublicError {
            code: "output_target_required".to_string(),
            message: "自定义输出目标不能为空。".to_string(),
        });
    }

    let mut config = state.config.lock().map_err(config_lock_error)?;
    let mut candidate = config.clone();
    match kind {
        TargetPresetKind::Replacement => {
            candidate.replacement_target_language = target_language;
            candidate.replacement_custom_target = custom_target;
        }
        TargetPresetKind::Translation => {
            candidate.translation_target_language = target_language;
            candidate.translation_custom_target = custom_target;
        }
    }

    persist_candidate(state.settings_path.as_deref(), &candidate)?;
    *config = candidate.clone();
    Ok(candidate)
}

#[tauri::command]
pub fn confirm_main_window_close(
    webview: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    behavior: CloseButtonBehavior,
) -> Result<RuntimePreferences, PublicError> {
    require_webview_label(&webview, &["main"])?;
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

    Ok(RuntimePreferences::from(&config))
}

#[tauri::command]
pub fn save_provider_config(
    webview: WebviewWindow,
    state: State<'_, AppState>,
    provider: ProviderUpdate,
) -> Result<SettingsConfigView, PublicError> {
    require_webview_label(&webview, &["main"])?;
    let config = save_provider_update_in_state(&state, provider)?;
    Ok(SettingsConfigView::from(&config))
}

pub fn save_provider_update_in_state(
    state: &AppState,
    update: ProviderUpdate,
) -> Result<AppConfig, PublicError> {
    let mut config = state.config.lock().map_err(config_lock_error)?;
    let provider = update.resolve(&config);
    let storage_provider_id = update.storage_provider_id().to_string();
    let original_provider_id = update
        .original_provider_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    validate_provider(&provider)?;
    validate_provider_id_available(&config, &provider.id, original_provider_id)?;
    save_provider_locked(
        &mut config,
        state.settings_path.as_deref(),
        provider,
        &storage_provider_id,
    )
}

pub fn save_provider_config_in_state(
    state: &AppState,
    provider: AiProviderConfig,
) -> Result<AppConfig, PublicError> {
    validate_provider(&provider)?;
    let storage_provider_id = provider.id.clone();
    let mut config = state.config.lock().map_err(config_lock_error)?;
    save_provider_locked(
        &mut config,
        state.settings_path.as_deref(),
        provider,
        &storage_provider_id,
    )
}

fn save_provider_locked(
    config: &mut AppConfig,
    settings_path: Option<&Path>,
    provider: AiProviderConfig,
    storage_provider_id: &str,
) -> Result<AppConfig, PublicError> {
    let mut candidate = config.clone();
    if let Some(existing) = candidate
        .providers
        .iter_mut()
        .find(|item| item.id == storage_provider_id)
    {
        *existing = provider.clone();
    } else {
        candidate.providers.push(provider.clone());
    }

    candidate.default_provider_id = Some(provider.id);
    persist_candidate(settings_path, &candidate)?;

    *config = candidate.clone();
    Ok(candidate)
}

fn validate_provider_id_available(
    config: &AppConfig,
    provider_id: &str,
    original_provider_id: Option<&str>,
) -> Result<(), PublicError> {
    let has_conflict = config.providers.iter().any(|provider| {
        provider.id == provider_id && Some(provider.id.as_str()) != original_provider_id
    });
    if has_conflict {
        return Err(PublicError {
            code: "provider_id_conflict".to_string(),
            message: format!("Provider with id '{provider_id}' already exists."),
        });
    }

    Ok(())
}

fn validate_provider(provider: &AiProviderConfig) -> Result<(), PublicError> {
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

    Ok(())
}

#[tauri::command]
pub fn set_default_provider(
    webview: WebviewWindow,
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<SettingsConfigView, PublicError> {
    require_webview_label(&webview, &["main"])?;
    let mut config = state.config.lock().map_err(config_lock_error)?;

    if !config.providers.iter().any(|p| p.id == provider_id) {
        return Err(PublicError {
            code: "provider_not_found".to_string(),
            message: format!("Provider with id '{}' not found.", provider_id),
        });
    }

    let mut candidate = config.clone();
    candidate.default_provider_id = Some(provider_id);
    persist_candidate(state.settings_path.as_deref(), &candidate)?;

    *config = candidate.clone();
    Ok(SettingsConfigView::from(&candidate))
}

#[tauri::command]
pub fn delete_provider(
    webview: WebviewWindow,
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<SettingsConfigView, PublicError> {
    require_webview_label(&webview, &["main"])?;
    let mut config = state.config.lock().map_err(config_lock_error)?;

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

    persist_candidate(state.settings_path.as_deref(), &candidate)?;

    *config = candidate.clone();
    Ok(SettingsConfigView::from(&candidate))
}

fn persist_candidate(
    settings_path: Option<&Path>,
    candidate: &AppConfig,
) -> Result<(), PublicError> {
    if let Some(path) = settings_path {
        candidate.save_to_path(path).map_err(|err| PublicError {
            code: "config_save_failed".to_string(),
            message: format!("Failed to save settings: {err}"),
        })?;
    }
    Ok(())
}

fn config_lock_error(
    err: std::sync::PoisonError<std::sync::MutexGuard<'_, AppConfig>>,
) -> PublicError {
    PublicError {
        code: "config_lock_failed".to_string(),
        message: err.to_string(),
    }
}
