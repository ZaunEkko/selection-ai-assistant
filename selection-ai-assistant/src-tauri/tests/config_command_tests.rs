use selection_ai_assistant_lib::app_state::AppState;
use selection_ai_assistant_lib::commands::config::{get_config_from_state, save_provider_config_in_state};
use selection_ai_assistant_lib::config::{AiProviderConfig, AppConfig};

fn provider(id: &str, base_url: &str, model: &str) -> AiProviderConfig {
    AiProviderConfig {
        id: id.to_string(),
        name: "Test Provider".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        api_key_ref: format!("credential://{id}"),
        headers: Vec::new(),
    }
}

#[test]
fn save_provider_config_adds_provider_and_sets_default() {
    let state = AppState::new(AppConfig::default());

    let config = save_provider_config_in_state(&state, provider("openai", "https://api.openai.com/v1", "gpt-test"))
        .expect("provider should save");

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].id, "openai");
    assert_eq!(config.default_provider_id.as_deref(), Some("openai"));
}

#[test]
fn save_provider_config_updates_existing_provider() {
    let state = AppState::new(AppConfig::default());
    save_provider_config_in_state(&state, provider("openai", "https://api.openai.com/v1", "gpt-test"))
        .expect("provider should save");

    let config = save_provider_config_in_state(&state, provider("openai", "https://example.com/v1", "gpt-next"))
        .expect("provider should update");

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].base_url, "https://example.com/v1");
    assert_eq!(config.providers[0].model, "gpt-next");
}

#[test]
fn save_provider_config_rejects_missing_required_fields() {
    let state = AppState::new(AppConfig::default());

    let err = save_provider_config_in_state(&state, provider("", "https://api.openai.com/v1", "gpt-test"))
        .expect_err("missing id should fail");
    assert_eq!(err.code, "provider_id_required");

    let err = save_provider_config_in_state(&state, provider("openai", "", "gpt-test"))
        .expect_err("missing base URL should fail");
    assert_eq!(err.code, "provider_config_incomplete");
}

#[test]
fn get_config_from_state_returns_current_config() {
    let state = AppState::new(AppConfig::default());
    save_provider_config_in_state(&state, provider("openai", "https://api.openai.com/v1", "gpt-test"))
        .expect("provider should save");

    let config = get_config_from_state(&state).expect("config should load");

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].id, "openai");
}
