use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Mutex, OnceLock};
use std::thread;

use selection_ai_assistant_lib::ai::action_classifier::AiAction;
use selection_ai_assistant_lib::commands::ai::{
    build_prompt_messages, list_provider_models, stream_chat_events_for_request,
    test_provider_connection, AiStreamErrorPayload,
};
use selection_ai_assistant_lib::config::AiProviderConfig;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvVarGuard {
    name: &'static str,
    previous: Option<String>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl EnvVarGuard {
    fn remove(name: &'static str) -> Self {
        let lock = env_lock().lock().expect("env lock should not be poisoned");
        let previous = env::var(name).ok();
        env::remove_var(name);
        Self {
            name,
            previous,
            _lock: lock,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            env::set_var(self.name, value);
        } else {
            env::remove_var(self.name);
        }
    }
}

fn provider(base_url: String, api_key: &str) -> AiProviderConfig {
    AiProviderConfig {
        id: "test".to_string(),
        name: "Test".to_string(),
        base_url,
        model: "".to_string(),
        api_key: api_key.to_string(),
        api_key_ref: "credential://test".to_string(),
        headers: vec![("X-Test".to_string(), "yes".to_string())],
    }
}

fn spawn_models_server(status_line: &'static str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server should have addr");
    thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("test server should accept request");
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).expect("request should read");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.starts_with("GET /v1/models HTTP/1.1"));
        assert!(
            request.contains("authorization: Bearer dummy-api-key")
                || request.contains("Authorization: Bearer dummy-api-key")
        );
        assert!(request.contains("x-test: yes") || request.contains("X-Test: yes"));
        let response = format!(
            "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("response should write");
    });
    format!("http://{addr}/v1")
}

fn spawn_chat_server(status_line: &'static str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server should have addr");
    thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("test server should accept request");
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).expect("request should read");
        let request = String::from_utf8_lossy(&request[..read]);
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(
            request.contains("authorization: Bearer dummy-api-key")
                || request.contains("Authorization: Bearer dummy-api-key")
        );
        let response = format!(
            "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("response should write");
    });
    format!("http://{addr}/v1")
}

#[tokio::test]
async fn stream_chat_failure_emits_error_then_done_without_leaking_secret() {
    let base_url = spawn_chat_server(
        "HTTP/1.1 401 Unauthorized",
        r#"{"error":{"message":"invalid API key dummy-api-key header-secret"}}"#,
    );
    let mut provider = provider(base_url, "dummy-api-key");
    provider.model = "gpt-test".to_string();
    provider
        .headers
        .push(("X-API-Key".to_string(), "header-secret".to_string()));

    let events = std::sync::Mutex::new(Vec::<String>::new());
    let messages = build_prompt_messages(AiAction::Explain, "hello world");

    stream_chat_events_for_request(
        provider,
        "dummy-api-key".to_string(),
        "request-1".to_string(),
        messages,
        |request_id, delta| {
            events
                .lock()
                .expect("events mutex should not be poisoned")
                .push(format!("delta:{request_id}:{delta}"));
        },
        |payload: AiStreamErrorPayload| {
            events
                .lock()
                .expect("events mutex should not be poisoned")
                .push(format!(
                    "error:{}:{}:{}",
                    payload.request_id, payload.code, payload.message
                ));
        },
        |request_id| {
            events
                .lock()
                .expect("events mutex should not be poisoned")
                .push(format!("done:{request_id}"));
        },
    )
    .await;

    let events = events
        .into_inner()
        .expect("events mutex should not be poisoned");
    assert_eq!(events.len(), 2);
    assert!(events[0].starts_with("error:request-1:provider_stream_failed:"));
    assert!(events[0].contains("HTTP 401"));
    assert!(events[0].contains("[redacted]"));
    assert!(!events[0].contains("dummy-api-key"));
    assert!(!events[0].contains("header-secret"));
    assert_eq!(events[1], "done:request-1");
}

#[tokio::test]
async fn list_provider_models_returns_model_ids_from_openai_compatible_endpoint() {
    let base_url = spawn_models_server(
        "HTTP/1.1 200 OK",
        r#"{"data":[{"id":"gpt-test"},{"id":"gpt-next"}]}"#,
    );

    let models = list_provider_models(provider(base_url, "dummy-api-key"))
        .await
        .expect("models should load");

    assert_eq!(models, vec!["gpt-test".to_string(), "gpt-next".to_string()]);
}

#[tokio::test]
async fn test_provider_connection_reports_model_count() {
    let base_url = spawn_models_server("HTTP/1.1 200 OK", r#"{"data":[{"id":"gpt-test"}]}"#);

    let result = test_provider_connection(provider(base_url, "dummy-api-key"))
        .await
        .expect("connection should succeed");

    assert!(result.success);
    assert_eq!(result.model_count, 1);
}

#[tokio::test]
async fn list_provider_models_surfaces_provider_error_body_without_leaking_secret() {
    let base_url = spawn_models_server(
        "HTTP/1.1 401 Unauthorized",
        r#"{"error":{"message":"invalid API key dummy-api-key header-secret"}}"#,
    );
    let mut provider = provider(base_url, "dummy-api-key");
    provider
        .headers
        .push(("X-API-Key".to_string(), "header-secret".to_string()));

    let err = list_provider_models(provider)
        .await
        .expect_err("provider error should fail");

    assert_eq!(err.code, "provider_model_list_failed");
    assert!(err.message.contains("HTTP 401"));
    assert!(err.message.contains("invalid API key"));
    assert!(err.message.contains("[redacted]"));
    assert!(!err.message.contains("dummy-api-key"));
    assert!(!err.message.contains("header-secret"));
}

#[tokio::test]
async fn list_provider_models_rejects_missing_api_key_without_leaking_secret() {
    let _env_guard = EnvVarGuard::remove("SELECTION_AI_API_KEY");
    let err = list_provider_models(provider("http://127.0.0.1:1/v1".to_string(), ""))
        .await
        .expect_err("missing key should fail");

    assert_eq!(err.code, "api_key_missing");
    assert!(!err.message.contains("dummy-api-key"));
}
