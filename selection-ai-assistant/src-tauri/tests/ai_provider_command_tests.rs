use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use selection_ai_assistant_lib::commands::ai::{list_provider_models, test_provider_connection};
use selection_ai_assistant_lib::config::AiProviderConfig;

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
async fn list_provider_models_rejects_missing_api_key_without_leaking_secret() {
    let err = list_provider_models(provider("http://127.0.0.1:1/v1".to_string(), ""))
        .await
        .expect_err("missing key should fail");

    assert_eq!(err.code, "api_key_missing");
    assert!(!err.message.contains("dummy-api-key"));
}
