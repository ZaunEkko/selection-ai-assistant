use std::time::Duration;

use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::AiProviderConfig;

const MODELS_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PROVIDER_ERROR_DETAIL_LEN: usize = 500;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VisionChatCompletionRequest {
    pub model: String,
    pub messages: Vec<VisionChatMessage>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VisionChatMessage {
    pub role: String,
    pub content: VisionChatContent,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum VisionChatContent {
    Text(String),
    Parts(Vec<VisionChatContentPart>),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VisionChatContentPart {
    Text { text: String },
    ImageUrl { image_url: VisionImageUrl },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VisionImageUrl {
    pub url: String,
}

pub fn build_vision_chat_request(
    model: impl Into<String>,
    system_prompt: impl Into<String>,
    user_prompt: impl Into<String>,
    image_data_url: impl Into<String>,
    stream: bool,
) -> VisionChatCompletionRequest {
    VisionChatCompletionRequest {
        model: model.into(),
        messages: vec![
            VisionChatMessage {
                role: "system".to_string(),
                content: VisionChatContent::Text(system_prompt.into()),
            },
            VisionChatMessage {
                role: "user".to_string(),
                content: VisionChatContent::Parts(vec![
                    VisionChatContentPart::Text {
                        text: user_prompt.into(),
                    },
                    VisionChatContentPart::ImageUrl {
                        image_url: VisionImageUrl {
                            url: image_data_url.into(),
                        },
                    },
                ]),
            },
        ],
        stream,
    }
}

pub fn build_chat_request(
    model: impl Into<String>,
    messages: Vec<ChatMessage>,
    stream: bool,
) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: model.into(),
        messages,
        stream,
    }
}

#[derive(Debug, Error)]
pub enum AiClientError {
    #[error("provider base URL is empty")]
    MissingBaseUrl,
    #[error("provider model is empty")]
    MissingModel,
    #[error("provider API key is empty")]
    MissingApiKey,
    #[error("invalid provider header name `{name}`: {message}")]
    InvalidHeaderName { name: String, message: String },
    #[error("invalid provider header value for `{name}`: {message}")]
    InvalidHeaderValue { name: String, message: String },
    #[error("request failed: {0}")]
    Request(String),
}

pub struct OpenAiCompatibleClient {
    http: reqwest::Client,
}

impl OpenAiCompatibleClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .build()
                .expect("OpenAI-compatible HTTP client should build"),
        }
    }

    pub fn endpoint(base_url: &str) -> Result<String, AiClientError> {
        let trimmed = base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(AiClientError::MissingBaseUrl);
        }
        Ok(format!("{trimmed}/chat/completions"))
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

    pub fn validate_provider(
        provider: &AiProviderConfig,
        api_key: &str,
    ) -> Result<(), AiClientError> {
        if provider.base_url.trim().is_empty() {
            return Err(AiClientError::MissingBaseUrl);
        }
        if provider.model.trim().is_empty() {
            return Err(AiClientError::MissingModel);
        }
        if api_key.trim().is_empty() {
            return Err(AiClientError::MissingApiKey);
        }
        Ok(())
    }

    pub fn validated_provider_headers(
        provider: &AiProviderConfig,
    ) -> Result<HeaderMap, AiClientError> {
        let mut headers = HeaderMap::new();

        for (name, value) in &provider.headers {
            let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|err| {
                AiClientError::InvalidHeaderName {
                    name: name.clone(),
                    message: err.to_string(),
                }
            })?;
            let header_value =
                HeaderValue::from_str(value).map_err(|err| AiClientError::InvalidHeaderValue {
                    name: name.clone(),
                    message: err.to_string(),
                })?;
            headers.insert(header_name, header_value);
        }

        Ok(headers)
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.http
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
        let headers = Self::validated_provider_headers(provider)?;
        let response = self
            .http
            .get(endpoint)
            .timeout(MODELS_REQUEST_TIMEOUT)
            .bearer_auth(api_key)
            .headers(headers)
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
        Self::validate_provider(provider, api_key)?;
        let endpoint = Self::endpoint(&provider.base_url)?;
        let body = build_chat_request(provider.model.clone(), messages, true);
        let headers = Self::validated_provider_headers(provider)?;

        self.stream_chat_request(provider, endpoint, headers, api_key, body, &mut on_delta)
            .await
    }

    pub async fn stream_vision_chat<F>(
        &self,
        provider: &AiProviderConfig,
        api_key: &str,
        system_prompt: &str,
        user_prompt: &str,
        image_data_url: String,
        mut on_delta: F,
    ) -> Result<(), AiClientError>
    where
        F: FnMut(String) + Send,
    {
        Self::validate_provider(provider, api_key)?;
        let endpoint = Self::endpoint(&provider.base_url)?;
        let body = build_vision_chat_request(
            provider.model.clone(),
            system_prompt,
            user_prompt,
            image_data_url,
            true,
        );
        let headers = Self::validated_provider_headers(provider)?;

        self.stream_chat_request(provider, endpoint, headers, api_key, body, &mut on_delta)
            .await
    }

    async fn stream_chat_request<F, B>(
        &self,
        provider: &AiProviderConfig,
        endpoint: String,
        headers: HeaderMap,
        api_key: &str,
        body: B,
        on_delta: &mut F,
    ) -> Result<(), AiClientError>
    where
        F: FnMut(String) + Send,
        B: Serialize,
    {
        let response = self
            .http
            .post(endpoint)
            .bearer_auth(api_key)
            .headers(headers)
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
            let parsed = extract_sse_deltas_from_bytes(&mut buffer, &bytes)?;

            for delta in parsed.deltas {
                on_delta(delta);
            }

            if parsed.done {
                return Ok(());
            }
        }

        if !buffer.is_empty() {
            let parsed = extract_sse_deltas_from_bytes(&mut buffer, b"\n")?;
            for delta in parsed.deltas {
                on_delta(delta);
            }
        }

        Ok(())
    }
}

async fn response_error(
    response: reqwest::Response,
    provider: &AiProviderConfig,
    api_key: &str,
) -> AiClientError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let detail = provider_error_detail(&body, &redaction_secrets(provider, api_key));
    let message = match detail {
        Some(detail) => format!("HTTP {status}: {detail}"),
        None => format!("HTTP {status}"),
    };
    AiClientError::Request(message)
}

fn redaction_secrets(provider: &AiProviderConfig, api_key: &str) -> Vec<String> {
    let mut secrets = Vec::new();
    let api_key = api_key.trim();
    if !api_key.is_empty() {
        secrets.push(api_key.to_string());
    }

    for (name, value) in &provider.headers {
        let value = value.trim();
        if !value.is_empty() && is_sensitive_header_name(name) {
            secrets.push(value.to_string());
        }
    }

    secrets
}

fn is_sensitive_header_name(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    normalized == "authorization"
        || normalized == "proxy-authorization"
        || normalized.contains("api-key")
        || normalized.contains("apikey")
        || normalized.contains("token")
        || normalized.contains("secret")
}

fn provider_error_detail(body: &str, secrets: &[String]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let message = value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(|message| message.as_str())?;
    let mut sanitized = message.split_whitespace().collect::<Vec<_>>().join(" ");
    for secret in secrets {
        sanitized = sanitized.replace(secret, "[redacted]");
    }
    let sanitized = sanitized.trim();
    if sanitized.is_empty() {
        return None;
    }
    Some(
        sanitized
            .chars()
            .take(MAX_PROVIDER_ERROR_DETAIL_LEN)
            .collect(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseParseResult {
    pub deltas: Vec<String>,
    pub done: bool,
}

pub fn extract_sse_deltas_from_bytes(
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
        if data.trim() == "[DONE]" {
            done = true;
            break;
        }

        if let Some(delta) = extract_delta_content(data) {
            deltas.push(delta);
        }
    }

    Ok(SseParseResult { deltas, done })
}

pub fn extract_delta_content(data: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(data).ok()?;
    value
        .get("choices")?
        .get(0)?
        .get("delta")?
        .get("content")?
        .as_str()
        .map(ToString::to_string)
}

impl Default for OpenAiCompatibleClient {
    fn default() -> Self {
        Self::new()
    }
}
