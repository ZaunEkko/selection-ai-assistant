use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_key_ref: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub default_provider_id: Option<String>,
    pub providers: Vec<AiProviderConfig>,
    pub hover_radius: f64,
    pub hover_delay_ms: u64,
    pub candidate_timeout_ms: u64,
    pub min_drag_distance: f64,
    pub hotkey: String,
    pub clipboard_fallback_enabled: bool,
    pub show_clipboard_privacy_warning_on_first_use: bool,
    pub disable_in_elevated_windows: bool,
    pub manual_hotkey_always_enabled: bool,
    pub disabled_apps: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_provider_id: None,
            providers: Vec::new(),
            hover_radius: 90.0,
            hover_delay_ms: 220,
            candidate_timeout_ms: 4_000,
            min_drag_distance: 6.0,
            hotkey: "Ctrl+Alt+A".to_string(),
            clipboard_fallback_enabled: true,
            show_clipboard_privacy_warning_on_first_use: true,
            disable_in_elevated_windows: true,
            manual_hotkey_always_enabled: true,
            disabled_apps: vec![
                "1Password.exe".to_string(),
                "KeePassXC.exe".to_string(),
                "Bitwarden.exe".to_string(),
                "mstsc.exe".to_string(),
                "AnyDesk.exe".to_string(),
                "TeamViewer.exe".to_string(),
            ],
        }
    }
}

impl AppConfig {
    pub fn is_disabled_process(&self, process_name: &str) -> bool {
        self.disabled_apps
            .iter()
            .any(|name| name.eq_ignore_ascii_case(process_name))
    }
}
