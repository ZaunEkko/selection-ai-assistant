use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::AiProviderConfig;

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
            http: reqwest::Client::new(),
        }
    }

    pub fn endpoint(base_url: &str) -> Result<String, AiClientError> {
        let trimmed = base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(AiClientError::MissingBaseUrl);
        }
        Ok(format!("{trimmed}/chat/completions"))
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
            return Err(AiClientError::Request(format!(
                "HTTP {}",
                response.status()
            )));
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
