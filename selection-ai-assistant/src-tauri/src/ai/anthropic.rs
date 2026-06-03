use std::time::Duration;

use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Serialize;

use crate::ai::openai_compatible::{
    AiClientError, ChatMessage, OpenAiCompatibleClient, SseParseResult,
};
use crate::config::AiProviderConfig;

const MODELS_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 2048;

pub struct AnthropicClient {
    http: reqwest::Client,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct AnthropicMessagesRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    stream: bool,
}

impl AnthropicClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .build()
                .expect("Anthropic HTTP client should build"),
        }
    }

    pub fn endpoint(base_url: &str) -> Result<String, AiClientError> {
        let trimmed = base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(AiClientError::MissingBaseUrl);
        }
        Ok(format!("{trimmed}/messages"))
    }

    pub fn models_endpoint(base_url: &str) -> Result<String, AiClientError> {
        let trimmed = base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(AiClientError::MissingBaseUrl);
        }
        Ok(format!("{trimmed}/models"))
    }

    pub fn parse_model_ids(body: &str) -> Result<Vec<String>, AiClientError> {
        let value: serde_json::Value = serde_json::from_str(body)
            .map_err(|err| AiClientError::Request(format!("invalid model list JSON: {err}")))?;
        let data = value
            .get("data")
            .and_then(|data| data.as_array())
            .ok_or_else(|| {
                AiClientError::Request("model list response missing data array".to_string())
            })?;

        Ok(data
            .iter()
            .filter_map(|item| item.get("id").and_then(|id| id.as_str()))
            .map(ToString::to_string)
            .collect())
    }

    pub async fn list_models(
        &self,
        provider: &AiProviderConfig,
        api_key: &str,
    ) -> Result<Vec<String>, AiClientError> {
        if provider.base_url.trim().is_empty() {
            return Err(AiClientError::MissingBaseUrl);
        }
        if api_key.trim().is_empty() {
            return Err(AiClientError::MissingApiKey);
        }

        let endpoint = Self::models_endpoint(&provider.base_url)?;
        let response = self
            .http
            .get(endpoint)
            .timeout(MODELS_REQUEST_TIMEOUT)
            .headers(Self::headers(provider, api_key)?)
            .send()
            .await
            .map_err(|err| AiClientError::Request(err.to_string()))?;

        if !response.status().is_success() {
            return Err(response_error(response, provider, api_key).await);
        }

        let body = response
            .text()
            .await
            .map_err(|err| AiClientError::Request(err.to_string()))?;
        Self::parse_model_ids(&body)
    }

    pub async fn stream_chat<F>(
        &self,
        provider: &AiProviderConfig,
        api_key: &str,
        messages: Vec<ChatMessage>,
        mut on_delta: F,
    ) -> Result<(), AiClientError>
    where
        F: FnMut(String) + Send,
    {
        OpenAiCompatibleClient::validate_provider(provider, api_key)?;
        let endpoint = Self::endpoint(&provider.base_url)?;
        let body = build_messages_request(provider.model.clone(), messages, true);

        let response = self
            .http
            .post(endpoint)
            .headers(Self::headers(provider, api_key)?)
            .json(&body)
            .send()
            .await
            .map_err(|err| AiClientError::Request(err.to_string()))?;

        if !response.status().is_success() {
            return Err(response_error(response, provider, api_key).await);
        }

        let mut buffer = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|err| AiClientError::Request(err.to_string()))?;
            let parsed = extract_anthropic_sse_deltas_from_bytes(&mut buffer, &bytes)
                .map_err(|err| sanitize_stream_error(err, provider, api_key))?;

            for delta in parsed.deltas {
                on_delta(delta);
            }

            if parsed.done {
                return Ok(());
            }
        }

        Ok(())
    }

    fn headers(provider: &AiProviderConfig, api_key: &str) -> Result<HeaderMap, AiClientError> {
        let mut headers = OpenAiCompatibleClient::validated_provider_headers(provider)?;
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(api_key).map_err(|err| AiClientError::InvalidHeaderValue {
                name: "x-api-key".to_string(),
                message: err.to_string(),
            })?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        Ok(headers)
    }
}

fn build_messages_request(
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
) -> AnthropicMessagesRequest {
    let mut system_parts = Vec::new();
    let mut anthropic_messages = Vec::new();

    for message in messages {
        match message.role.as_str() {
            "system" => system_parts.push(message.content),
            "assistant" => anthropic_messages.push(AnthropicMessage {
                role: "assistant".to_string(),
                content: message.content,
            }),
            _ => anthropic_messages.push(AnthropicMessage {
                role: "user".to_string(),
                content: message.content,
            }),
        }
    }

    AnthropicMessagesRequest {
        model,
        max_tokens: MAX_TOKENS,
        system: (!system_parts.is_empty()).then(|| system_parts.join("\n\n")),
        messages: anthropic_messages,
        stream,
    }
}

pub fn extract_anthropic_sse_deltas_from_bytes(
    buffer: &mut Vec<u8>,
    chunk: &[u8],
) -> Result<SseParseResult, AiClientError> {
    buffer.extend_from_slice(chunk);

    let mut deltas = Vec::new();
    let mut done = false;

    while let Some(newline_index) = buffer.iter().position(|byte| *byte == b'\n') {
        let mut line_bytes: Vec<u8> = buffer.drain(..=newline_index).collect();
        if line_bytes.last() == Some(&b'\n') {
            line_bytes.pop();
        }
        if line_bytes.last() == Some(&b'\r') {
            line_bytes.pop();
        }

        let line = std::str::from_utf8(&line_bytes)
            .map_err(|err| AiClientError::Request(format!("invalid UTF-8 in SSE line: {err}")))?;

        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim_start();
        if let Some(message) = extract_anthropic_error_message(data) {
            return Err(AiClientError::Request(format!(
                "Anthropic stream error: {message}"
            )));
        }
        if let Some(delta) = extract_anthropic_delta_content(data) {
            deltas.push(delta);
        }
        if is_anthropic_done(data) {
            done = true;
            break;
        }
    }

    Ok(SseParseResult { deltas, done })
}

pub fn extract_anthropic_delta_content(data: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(data).ok()?;
    if value.get("type")?.as_str()? != "content_block_delta" {
        return None;
    }
    let delta = value.get("delta")?;
    if delta.get("type")?.as_str()? != "text_delta" {
        return None;
    }
    delta.get("text")?.as_str().map(ToString::to_string)
}

fn extract_anthropic_error_message(data: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(data).ok()?;
    if value.get("type")?.as_str()? != "error" {
        return None;
    }
    provider_error_message(data)
}

fn is_anthropic_done(data: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(data) else {
        return false;
    };
    value.get("type").and_then(|item| item.as_str()) == Some("message_stop")
}

fn sanitize_stream_error(
    err: AiClientError,
    provider: &AiProviderConfig,
    api_key: &str,
) -> AiClientError {
    let AiClientError::Request(mut message) = err else {
        return err;
    };

    for secret in redaction_secrets(provider, api_key) {
        message = message.replace(&secret, "[redacted]");
    }

    AiClientError::Request(message)
}

async fn response_error(
    response: reqwest::Response,
    provider: &AiProviderConfig,
    api_key: &str,
) -> AiClientError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let mut message = provider_error_message(&body).unwrap_or_else(|| format!("HTTP {status}"));
    for secret in redaction_secrets(provider, api_key) {
        message = message.replace(&secret, "[redacted]");
    }
    if message.starts_with("HTTP ") {
        AiClientError::Request(message)
    } else {
        AiClientError::Request(format!("HTTP {status}: {message}"))
    }
}

fn provider_error_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(|message| message.as_str())
        .map(|message| message.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn redaction_secrets(provider: &AiProviderConfig, api_key: &str) -> Vec<String> {
    let mut secrets = Vec::new();
    let api_key = api_key.trim();
    if !api_key.is_empty() {
        secrets.push(api_key.to_string());
    }
    for (name, value) in &provider.headers {
        let normalized = name.trim().to_ascii_lowercase();
        if !value.trim().is_empty()
            && (normalized == "authorization"
                || normalized == "proxy-authorization"
                || normalized.contains("api-key")
                || normalized.contains("apikey")
                || normalized.contains("token")
                || normalized.contains("secret"))
        {
            secrets.push(value.trim().to_string());
        }
    }
    secrets
}

impl Default for AnthropicClient {
    fn default() -> Self {
        Self::new()
    }
}
