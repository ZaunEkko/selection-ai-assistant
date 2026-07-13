use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Mutex, OnceLock};
use std::thread;

use selection_ai_assistant_lib::ai::action_classifier::AiAction;
use selection_ai_assistant_lib::ai::openai_compatible::ChatMessage;
use selection_ai_assistant_lib::app_state::AppState;
use selection_ai_assistant_lib::commands::ai::{
    build_prompt_messages, list_provider_models_for_provider, list_provider_models_for_update,
    stream_chat_events_for_request, test_provider_connection_for_provider,
    test_provider_connection_for_update, AiStreamErrorPayload,
};
use selection_ai_assistant_lib::config::{
    AiProviderConfig, AiProviderKind, AppConfig, ProviderUpdate, SecretUpdate,
};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

async fn provider_test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
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
        provider_kind: AiProviderKind::OpenAiCompatible,
        api_key: api_key.to_string(),
        api_key_ref: "credential://test".to_string(),
        headers: vec![("X-Test".to_string(), "base-header-secret".to_string())],
    }
}

fn provider_update(base_url: String) -> ProviderUpdate {
    ProviderUpdate {
        original_provider_id: Some("test".to_string()),
        id: "test".to_string(),
        name: "Updated Test".to_string(),
        base_url,
        model: "".to_string(),
        provider_kind: AiProviderKind::OpenAiCompatible,
        api_key: SecretUpdate::Keep,
        api_key_ref: "credential://test".to_string(),
    }
}

fn state_with_saved_provider() -> AppState {
    AppState::new(AppConfig {
        default_provider_id: Some("test".to_string()),
        providers: vec![provider(
            "https://saved.example/v1".to_string(),
            "dummy-api-key",
        )],
        ..AppConfig::default()
    })
}

fn read_http_request(stream: &mut impl Read) -> String {
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;

    loop {
        let read = stream.read(&mut chunk).expect("request should read");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);

        if header_end.is_none() {
            header_end = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|index| index + 4);
        }

        if let Some(header_end) = header_end {
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if request.len() >= header_end + content_length {
                break;
            }
        }
    }

    String::from_utf8_lossy(&request).to_string()
}

fn spawn_models_server(status_line: &'static str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server should have addr");
    thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("test server should accept request");
        let request = read_http_request(&mut stream);
        assert!(request.starts_with("GET /v1/models HTTP/1.1"));
        assert!(
            request.contains("authorization: Bearer dummy-api-key")
                || request.contains("Authorization: Bearer dummy-api-key")
        );
        assert!(
            request.contains("x-test: base-header-secret")
                || request.contains("X-Test: base-header-secret")
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

fn spawn_chat_server(status_line: &'static str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server should have addr");
    thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("test server should accept request");
        let request = read_http_request(&mut stream);
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

fn spawn_models_then_chat_server(
    models_status_line: &'static str,
    models_body: &'static str,
    chat_status_line: &'static str,
    chat_body: &'static str,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server should have addr");
    thread::spawn(move || {
        let (mut models_stream, _) = listener
            .accept()
            .expect("test server should accept models request");
        let request = read_http_request(&mut models_stream);
        assert!(request.starts_with("GET /v1/models HTTP/1.1"));
        let response = format!(
            "{models_status_line}\r\ncontent-type: application/json\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{models_body}",
            models_body.len()
        );
        models_stream
            .write_all(response.as_bytes())
            .expect("models response should write");
        drop(models_stream);

        let (mut chat_stream, _) = listener
            .accept()
            .expect("test server should accept chat request");
        let request = read_http_request(&mut chat_stream);
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
        let response = format!(
            "{chat_status_line}\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n{chat_body}",
            chat_body.len()
        );
        chat_stream
            .write_all(response.as_bytes())
            .expect("chat response should write");
    });
    format!("http://{addr}/v1")
}

fn spawn_anthropic_messages_server(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server should have addr");
    thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("test server should accept request");
        let request = read_http_request(&mut stream);
        assert!(request.starts_with("POST /v1/messages HTTP/1.1"));
        assert!(
            request.contains("x-api-key: dummy-api-key")
                || request.contains("X-Api-Key: dummy-api-key")
        );
        assert!(
            request.contains("anthropic-version: 2023-06-01")
                || request.contains("Anthropic-Version: 2023-06-01")
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("response should write");
    });
    format!("http://{addr}/v1")
}

fn spawn_gemini_stream_server(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server should have addr");
    thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("test server should accept request");
        let request = read_http_request(&mut stream);
        assert!(request
            .starts_with("POST /v1beta/models/gemini-test:streamGenerateContent?alt=sse HTTP/1.1"));
        assert!(
            request.contains("x-goog-api-key: dummy-api-key")
                || request.contains("X-Goog-Api-Key: dummy-api-key")
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("response should write");
    });
    format!("http://{addr}/v1beta")
}

fn spawn_gemini_stream_error_server(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server should have addr");
    thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("test server should accept request");
        let request = read_http_request(&mut stream);
        assert!(request
            .starts_with("POST /v1beta/models/gemini-test:streamGenerateContent?alt=sse HTTP/1.1"));
        let response = format!(
            "HTTP/1.1 401 Unauthorized\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("response should write");
    });
    format!("http://{addr}/v1beta")
}

#[tokio::test]
async fn stream_chat_supports_anthropic_messages_api() {
    let _guard = provider_test_lock().await;
    let base_url = spawn_anthropic_messages_server(
        "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Claude OK\"}}\n\ndata: {\"type\":\"message_stop\"}\n\n",
    );
    let mut provider = provider(base_url, "dummy-api-key");
    provider.provider_kind = AiProviderKind::Anthropic;
    provider.model = "claude-sonnet-4-6".to_string();

    let events = std::sync::Mutex::new(Vec::<String>::new());
    let errors = std::sync::Mutex::new(Vec::<String>::new());
    stream_chat_events_for_request(
        provider,
        "dummy-api-key".to_string(),
        "request-claude".to_string(),
        vec![
            ChatMessage::system("system prompt"),
            ChatMessage::user("hello"),
        ],
        |_, delta| events.lock().expect("events mutex").push(delta),
        |payload| errors.lock().expect("errors mutex").push(payload.message),
        |_| {},
    )
    .await;

    assert_eq!(
        errors.into_inner().expect("errors mutex"),
        Vec::<String>::new()
    );
    assert_eq!(
        events.into_inner().expect("events mutex"),
        vec!["Claude OK".to_string()]
    );
}

#[tokio::test]
async fn anthropic_sse_error_event_emits_stream_error_without_leaking_secret() {
    let _guard = provider_test_lock().await;
    let base_url = spawn_anthropic_messages_server(
        "event: error\ndata: {\"type\":\"error\",\"error\":{\"message\":\"rate limit dummy-api-key base-header-secret header-secret trace-secret\"}}\n\n",
    );
    let mut provider = provider(base_url, "dummy-api-key");
    provider.provider_kind = AiProviderKind::Anthropic;
    provider.model = "claude-sonnet-4-6".to_string();
    provider
        .headers
        .push(("X-API-Key".to_string(), "header-secret".to_string()));
    provider
        .headers
        .push(("X-Trace-Context".to_string(), "trace-secret".to_string()));

    let events = std::sync::Mutex::new(Vec::<String>::new());
    stream_chat_events_for_request(
        provider,
        "dummy-api-key".to_string(),
        "request-claude-error".to_string(),
        vec![ChatMessage::user("hello")],
        |request_id, delta| {
            events
                .lock()
                .expect("events mutex")
                .push(format!("delta:{request_id}:{delta}"));
        },
        |payload| {
            events.lock().expect("events mutex").push(format!(
                "error:{}:{}:{}",
                payload.request_id, payload.code, payload.message
            ));
        },
        |request_id| {
            events
                .lock()
                .expect("events mutex")
                .push(format!("done:{request_id}"));
        },
    )
    .await;

    let events = events.into_inner().expect("events mutex");
    assert_eq!(events.len(), 2);
    assert!(events[0].starts_with("error:request-claude-error:provider_stream_failed:"));
    assert!(
        events[0].contains("rate limit"),
        "expected Anthropic SSE error detail, got {events:?}"
    );
    assert!(events[0].contains("[redacted]"));
    assert!(!events[0].contains("dummy-api-key"));
    assert!(!events[0].contains("base-header-secret"));
    assert!(!events[0].contains("header-secret"));
    assert!(!events[0].contains("trace-secret"));
    assert_eq!(events[1], "done:request-claude-error");
}

#[tokio::test]
async fn stream_chat_supports_gemini_stream_generate_content_api() {
    let _guard = provider_test_lock().await;
    let base_url = spawn_gemini_stream_server(
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Gemini OK\"}]}}]}\n\n",
    );
    let mut provider = provider(base_url, "dummy-api-key");
    provider.provider_kind = AiProviderKind::Gemini;
    provider.model = "gemini-test".to_string();

    let events = std::sync::Mutex::new(Vec::<String>::new());
    let errors = std::sync::Mutex::new(Vec::<String>::new());
    stream_chat_events_for_request(
        provider,
        "dummy-api-key".to_string(),
        "request-gemini".to_string(),
        vec![
            ChatMessage::system("system prompt"),
            ChatMessage::user("hello"),
        ],
        |_, delta| events.lock().expect("events mutex").push(delta),
        |payload| errors.lock().expect("errors mutex").push(payload.message),
        |_| {},
    )
    .await;

    assert_eq!(
        errors.into_inner().expect("errors mutex"),
        Vec::<String>::new()
    );
    assert_eq!(
        events.into_inner().expect("events mutex"),
        vec!["Gemini OK".to_string()]
    );
}

#[tokio::test]
async fn gemini_http_error_redacts_api_key_and_all_custom_header_values() {
    let _guard = provider_test_lock().await;
    let base_url = spawn_gemini_stream_error_server(
        r#"{"error":{"message":"invalid credentials dummy-api-key base-header-secret header-secret trace-secret"}}"#,
    );
    let mut provider = provider(base_url, "dummy-api-key");
    provider.provider_kind = AiProviderKind::Gemini;
    provider.model = "gemini-test".to_string();
    provider
        .headers
        .push(("X-API-Key".to_string(), "header-secret".to_string()));
    provider
        .headers
        .push(("X-Trace-Context".to_string(), "trace-secret".to_string()));

    let events = std::sync::Mutex::new(Vec::<String>::new());
    stream_chat_events_for_request(
        provider,
        "dummy-api-key".to_string(),
        "request-gemini-error".to_string(),
        vec![ChatMessage::user("hello")],
        |request_id, delta| {
            events
                .lock()
                .expect("events mutex")
                .push(format!("delta:{request_id}:{delta}"));
        },
        |payload| {
            events.lock().expect("events mutex").push(format!(
                "error:{}:{}:{}",
                payload.request_id, payload.code, payload.message
            ));
        },
        |request_id| {
            events
                .lock()
                .expect("events mutex")
                .push(format!("done:{request_id}"));
        },
    )
    .await;

    let events = events.into_inner().expect("events mutex");
    assert_eq!(events.len(), 2);
    assert!(events[0].starts_with("error:request-gemini-error:provider_stream_failed:"));
    assert!(events[0].contains("HTTP 401"));
    assert!(events[0].contains("[redacted]"));
    assert!(!events[0].contains("dummy-api-key"));
    assert!(!events[0].contains("base-header-secret"));
    assert!(!events[0].contains("header-secret"));
    assert!(!events[0].contains("trace-secret"));
    assert_eq!(events[1], "done:request-gemini-error");
}

#[tokio::test]
async fn test_provider_connection_falls_back_to_chat_probe_when_models_endpoint_is_unavailable() {
    let _guard = provider_test_lock().await;
    let base_url = spawn_models_then_chat_server(
        "HTTP/1.1 404 Not Found",
        r#"{"error":{"message":"models endpoint not found"}}"#,
        "HTTP/1.1 200 OK",
        "data: {\"choices\":[{\"delta\":{\"content\":\"OK\"}}]}\n\ndata: [DONE]\n\n",
    );
    let mut provider = provider(base_url, "dummy-api-key");
    provider.model = "qwen-coder-test".to_string();

    let result = test_provider_connection_for_provider(provider)
        .await
        .expect("chat probe should allow providers without a models endpoint");

    assert!(result.success);
    assert_eq!(result.model_count, 0);
    assert!(!result.model_list_available);
}

#[tokio::test]
async fn stream_chat_failure_emits_error_then_done_without_leaking_secret() {
    let _guard = provider_test_lock().await;
    let base_url = spawn_chat_server(
        "HTTP/1.1 401 Unauthorized",
        r#"{"error":{"message":"invalid API key dummy-api-key base-header-secret header-secret trace-secret"}}"#,
    );
    let mut provider = provider(base_url, "dummy-api-key");
    provider.model = "gpt-test".to_string();
    provider
        .headers
        .push(("X-API-Key".to_string(), "header-secret".to_string()));
    provider
        .headers
        .push(("X-Trace-Context".to_string(), "trace-secret".to_string()));

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
    assert!(!events[0].contains("base-header-secret"));
    assert!(!events[0].contains("header-secret"));
    assert!(!events[0].contains("trace-secret"));
    assert_eq!(events[1], "done:request-1");
}

#[tokio::test]
async fn list_provider_models_for_update_resolves_saved_api_key_and_custom_headers() {
    let _guard = provider_test_lock().await;
    let base_url =
        spawn_models_server("HTTP/1.1 200 OK", r#"{"data":[{"id":"gpt-saved-secret"}]}"#);
    let state = state_with_saved_provider();

    let models = list_provider_models_for_update(&state, provider_update(base_url))
        .await
        .expect("models helper should resolve saved provider secrets");

    assert_eq!(models, vec!["gpt-saved-secret".to_string()]);
}

#[tokio::test]
async fn test_provider_connection_for_update_resolves_saved_api_key_and_custom_headers() {
    let _guard = provider_test_lock().await;
    let base_url =
        spawn_models_server("HTTP/1.1 200 OK", r#"{"data":[{"id":"gpt-saved-secret"}]}"#);
    let state = state_with_saved_provider();

    let result = test_provider_connection_for_update(&state, provider_update(base_url))
        .await
        .expect("connection helper should resolve saved provider secrets");

    assert!(result.success);
    assert_eq!(result.model_count, 1);
    assert!(result.model_list_available);
}

#[tokio::test]
async fn list_provider_models_returns_model_ids_from_openai_compatible_endpoint() {
    let _guard = provider_test_lock().await;
    let base_url = spawn_models_server(
        "HTTP/1.1 200 OK",
        r#"{"data":[{"id":"gpt-test"},{"id":"gpt-next"}]}"#,
    );

    let models = list_provider_models_for_provider(provider(base_url, "dummy-api-key"))
        .await
        .expect("models should load");

    assert_eq!(models, vec!["gpt-test".to_string(), "gpt-next".to_string()]);
}

#[tokio::test]
async fn test_provider_connection_reports_model_count() {
    let _guard = provider_test_lock().await;
    let base_url = spawn_models_server("HTTP/1.1 200 OK", r#"{"data":[{"id":"gpt-test"}]}"#);

    let result = test_provider_connection_for_provider(provider(base_url, "dummy-api-key"))
        .await
        .expect("connection should succeed");

    assert!(result.success);
    assert_eq!(result.model_count, 1);
    assert!(result.model_list_available);
}

#[tokio::test]
async fn list_provider_models_surfaces_provider_error_body_without_leaking_secret() {
    let _guard = provider_test_lock().await;
    let base_url = spawn_models_server(
        "HTTP/1.1 401 Unauthorized",
        r#"{"error":{"message":"invalid API key dummy-api-key base-header-secret header-secret trace-secret"}}"#,
    );
    let mut provider = provider(base_url, "dummy-api-key");
    provider
        .headers
        .push(("X-API-Key".to_string(), "header-secret".to_string()));
    provider
        .headers
        .push(("X-Trace-Context".to_string(), "trace-secret".to_string()));

    let err = list_provider_models_for_provider(provider)
        .await
        .expect_err("provider error should fail");

    assert_eq!(err.code, "provider_model_list_failed");
    assert!(err.message.contains("HTTP 401"));
    assert!(err.message.contains("invalid API key"));
    assert!(err.message.contains("[redacted]"));
    assert!(!err.message.contains("dummy-api-key"));
    assert!(!err.message.contains("base-header-secret"));
    assert!(!err.message.contains("header-secret"));
    assert!(!err.message.contains("trace-secret"));
}

#[tokio::test]
async fn list_provider_models_rejects_missing_api_key_without_leaking_secret() {
    let _guard = provider_test_lock().await;
    let _env_guard = EnvVarGuard::remove("SELECTION_AI_API_KEY");
    let err = list_provider_models_for_provider(provider("http://127.0.0.1:1/v1".to_string(), ""))
        .await
        .expect_err("missing key should fail");

    assert_eq!(err.code, "api_key_missing");
    assert!(!err.message.contains("dummy-api-key"));
}
