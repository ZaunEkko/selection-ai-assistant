use serde::{Deserialize, Serialize};

use super::store::{
    AiProviderConfig, AiProviderKind, AppConfig, CloseButtonBehavior, ReplacementTargetLanguage,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigView {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub provider_kind: AiProviderKind,
    pub api_key_configured: bool,
    pub api_key_ref: String,
    pub custom_headers_configured: bool,
}

impl From<&AiProviderConfig> for ProviderConfigView {
    fn from(provider: &AiProviderConfig) -> Self {
        Self {
            id: provider.id.clone(),
            name: provider.name.clone(),
            base_url: provider.base_url.clone(),
            model: provider.model.clone(),
            provider_kind: provider.provider_kind,
            api_key_configured: !provider.api_key.trim().is_empty(),
            api_key_ref: provider.api_key_ref.clone(),
            custom_headers_configured: !provider.headers.is_empty(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsConfigView {
    pub default_provider_id: Option<String>,
    pub providers: Vec<ProviderConfigView>,
    pub hotkey: String,
    pub launch_at_startup: bool,
    pub clipboard_fallback_enabled: bool,
    pub start_minimized_to_tray: bool,
    pub close_button_behavior: CloseButtonBehavior,
    pub replacement_target_language: ReplacementTargetLanguage,
    pub replacement_custom_target: String,
    pub translation_target_language: ReplacementTargetLanguage,
    pub translation_custom_target: String,
    pub disabled_apps: Vec<String>,
}

impl From<&AppConfig> for SettingsConfigView {
    fn from(config: &AppConfig) -> Self {
        Self {
            default_provider_id: config.default_provider_id.clone(),
            providers: config
                .providers
                .iter()
                .map(ProviderConfigView::from)
                .collect(),
            hotkey: config.hotkey.clone(),
            launch_at_startup: config.launch_at_startup,
            clipboard_fallback_enabled: config.clipboard_fallback_enabled,
            start_minimized_to_tray: config.start_minimized_to_tray,
            close_button_behavior: config.close_button_behavior,
            replacement_target_language: config.replacement_target_language,
            replacement_custom_target: config.replacement_custom_target.clone(),
            translation_target_language: config.translation_target_language,
            translation_custom_target: config.translation_custom_target.clone(),
            disabled_apps: config.disabled_apps.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePreferences {
    pub hotkey: String,
    pub launch_at_startup: bool,
    pub start_minimized_to_tray: bool,
    pub close_button_behavior: CloseButtonBehavior,
    pub replacement_target_language: ReplacementTargetLanguage,
    pub replacement_custom_target: String,
    pub translation_target_language: ReplacementTargetLanguage,
    pub translation_custom_target: String,
}

impl From<&AppConfig> for RuntimePreferences {
    fn from(config: &AppConfig) -> Self {
        Self {
            hotkey: config.hotkey.clone(),
            launch_at_startup: config.launch_at_startup,
            start_minimized_to_tray: config.start_minimized_to_tray,
            close_button_behavior: config.close_button_behavior,
            replacement_target_language: config.replacement_target_language,
            replacement_custom_target: config.replacement_custom_target.clone(),
            translation_target_language: config.translation_target_language,
            translation_custom_target: config.translation_custom_target.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum SecretUpdate {
    Keep,
    Replace { value: String },
    Clear,
}

impl Default for SecretUpdate {
    fn default() -> Self {
        Self::Keep
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpdate {
    pub original_provider_id: Option<String>,
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub provider_kind: AiProviderKind,
    pub api_key: SecretUpdate,
    pub api_key_ref: String,
}

impl ProviderUpdate {
    pub fn resolve(&self, config: &AppConfig) -> AiProviderConfig {
        let existing = self.existing_provider(config);
        let api_key = match &self.api_key {
            SecretUpdate::Keep => existing
                .map(|provider| provider.api_key.clone())
                .unwrap_or_default(),
            SecretUpdate::Replace { value } => value.clone(),
            SecretUpdate::Clear => String::new(),
        };
        let headers = existing
            .map(|provider| provider.headers.clone())
            .unwrap_or_default();

        AiProviderConfig {
            id: self.id.clone(),
            name: self.name.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            provider_kind: self.provider_kind,
            api_key,
            api_key_ref: self.api_key_ref.clone(),
            headers,
        }
    }

    pub fn storage_provider_id(&self) -> &str {
        self.original_provider_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&self.id)
    }

    fn existing_provider<'a>(&self, config: &'a AppConfig) -> Option<&'a AiProviderConfig> {
        let provider_id = self.storage_provider_id();
        config
            .providers
            .iter()
            .find(|provider| provider.id == provider_id)
    }
}
