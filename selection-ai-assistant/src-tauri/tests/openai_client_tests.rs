use selection_ai_assistant_lib::ai::openai_compatible::{
    build_chat_request, build_vision_chat_request, extract_delta_content,
    extract_sse_deltas_from_bytes, AiClientError, ChatMessage, OpenAiCompatibleClient,
};
use selection_ai_assistant_lib::config::{AiProviderConfig, AiProviderKind};

#[test]
fn builds_openai_compatible_chat_request() {
    let request = build_chat_request(
        "gpt-test",
        vec![
            ChatMessage::system("你是一个桌面划词 AI 助手"),
            ChatMessage::user("解释这句话"),
        ],
        true,
    );

    assert_eq!(request.model, "gpt-test");
    assert!(request.stream);
    assert_eq!(request.messages.len(), 2);
    assert_eq!(request.messages[0].role, "system");
    assert_eq!(request.messages[1].role, "user");
}

#[test]
fn builds_openai_compatible_vision_chat_request() {
    let request = build_vision_chat_request(
        "vision-test",
        "你是截图翻译助手",
        "翻译截图中的文字",
        "data:image/png;base64,abc123",
        true,
    );
    let value = serde_json::to_value(request).expect("vision request should serialize");

    assert_eq!(value["model"], "vision-test");
    assert_eq!(value["stream"], true);
    assert_eq!(value["messages"][0]["role"], "system");
    assert_eq!(value["messages"][0]["content"], "你是截图翻译助手");
    assert_eq!(value["messages"][1]["role"], "user");
    assert_eq!(value["messages"][1]["content"][0]["type"], "text");
    assert_eq!(
        value["messages"][1]["content"][0]["text"],
        "翻译截图中的文字"
    );
    assert_eq!(value["messages"][1]["content"][1]["type"], "image_url");
    assert_eq!(
        value["messages"][1]["content"][1]["image_url"]["url"],
        "data:image/png;base64,abc123"
    );
}

#[test]
fn creates_chat_completion_endpoint_from_base_url() {
    let endpoint = OpenAiCompatibleClient::endpoint("https://api.openai.com/v1/").unwrap();
    assert_eq!(endpoint, "https://api.openai.com/v1/chat/completions");
}

#[test]
fn creates_models_endpoint_from_base_url() {
    let endpoint = OpenAiCompatibleClient::models_endpoint("https://api.openai.com/v1/").unwrap();
    assert_eq!(endpoint, "https://api.openai.com/v1/models");
}

#[test]
fn parses_model_ids_from_models_response() {
    let ids = OpenAiCompatibleClient::parse_model_ids(
        r#"{"data":[{"id":"gpt-4.1"},{"id":"gpt-4.1-mini"}]}"#,
    )
    .expect("models should parse");

    assert_eq!(ids, vec!["gpt-4.1".to_string(), "gpt-4.1-mini".to_string()]);
}
#[test]
fn extracts_stream_delta_content() {
    let data = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;
    assert_eq!(extract_delta_content(data), Some("hello".to_string()));
}

#[test]
fn ignores_done_and_non_content_stream_payloads() {
    assert_eq!(extract_delta_content("[DONE]"), None);
    assert_eq!(
        extract_delta_content(r#"{"choices":[{"delta":{"role":"assistant"}}]}"#),
        None
    );
}

#[test]
fn validates_provider_headers() {
    let provider = AiProviderConfig {
        id: "test".to_string(),
        name: "Test".to_string(),
        base_url: "https://example.com/v1".to_string(),
        model: "gpt-test".to_string(),
        provider_kind: AiProviderKind::OpenAiCompatible,
        api_key: "test-api-key".to_string(),
        api_key_ref: "credential://test".to_string(),
        headers: vec![("X-App".to_string(), "selection-ai".to_string())],
    };

    let headers = OpenAiCompatibleClient::validated_provider_headers(&provider).unwrap();

    assert_eq!(
        headers.get("x-app").unwrap().to_str().unwrap(),
        "selection-ai"
    );
}

#[test]
fn rejects_invalid_provider_header_names() {
    let provider = AiProviderConfig {
        id: "test".to_string(),
        name: "Test".to_string(),
        base_url: "https://example.com/v1".to_string(),
        model: "gpt-test".to_string(),
        provider_kind: AiProviderKind::OpenAiCompatible,
        api_key: "test-api-key".to_string(),
        api_key_ref: "credential://test".to_string(),
        headers: vec![("bad header".to_string(), "value".to_string())],
    };

    assert!(matches!(
        OpenAiCompatibleClient::validated_provider_headers(&provider),
        Err(AiClientError::InvalidHeaderName { .. })
    ));
}

#[test]
fn extracts_sse_deltas_from_split_utf8_chunks_without_lossy_replacement() {
    let line = r#"data: {"choices":[{"delta":{"content":"你好"}}]}

"#;
    let bytes = line.as_bytes();
    let split = bytes
        .windows("好".len())
        .position(|window| window == "好".as_bytes())
        .expect("test fixture should contain the 好 bytes")
        + 1;

    let mut buffer = Vec::new();
    let first = extract_sse_deltas_from_bytes(&mut buffer, &bytes[..split]).unwrap();
    let second = extract_sse_deltas_from_bytes(&mut buffer, &bytes[split..]).unwrap();

    assert!(first.deltas.is_empty());
    assert!(!first.done);
    assert_eq!(second.deltas, vec!["你好".to_string()]);
    assert!(!second.done);
}
