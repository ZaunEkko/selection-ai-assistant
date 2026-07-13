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

pub struct GeminiClient {
    http: reqwest::Client,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerateContentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    contents: Vec<GeminiContent>,
}

impl GeminiClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .build()
                .expect("Gemini HTTP client should build"),
        }
    }

    pub fn endpoint(base_url: &str, model: &str) -> Result<String, AiClientError> {
        let trimmed = base_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            return Err(AiClientError::MissingBaseUrl);
        }
        let model = normalized_model(model)?;
        Ok(format!(
            "{trimmed}/models/{model}:streamGenerateContent?alt=sse"
        ))
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
        let models = value
            .get("models")
            .and_then(|models| models.as_array())
            .ok_or_else(|| {
                AiClientError::Request("model list response missing models array".to_string())
            })?;

        Ok(models
            .iter()
            .filter_map(|item| item.get("name").and_then(|name| name.as_str()))
            .map(|name| name.strip_prefix("models/").unwrap_or(name).to_string())
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
        let endpoint = Self::endpoint(&provider.base_url, &provider.model)?;
        let body = build_generate_content_request(messages);

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
            let parsed = extract_gemini_sse_deltas_from_bytes(&mut buffer, &bytes)?;

            for delta in parsed.deltas {
                on_delta(delta);
            }
        }

        Ok(())
    }

    fn headers(provider: &AiProviderConfig, api_key: &str) -> Result<HeaderMap, AiClientError> {
        let mut headers = OpenAiCompatibleClient::validated_provider_headers(provider)?;
        headers.insert(
            "x-goog-api-key",
            HeaderValue::from_str(api_key).map_err(|err| AiClientError::InvalidHeaderValue {
                name: "x-goog-api-key".to_string(),
                message: err.to_string(),
            })?,
        );
        Ok(headers)
    }
}

fn normalized_model(model: &str) -> Result<String, AiClientError> {
    let model = model.trim().strip_prefix("models/").unwrap_or(model.trim());
    if model.is_empty() {
        return Err(AiClientError::MissingModel);
    }
    Ok(model.to_string())
}

fn build_generate_content_request(messages: Vec<ChatMessage>) -> GeminiGenerateContentRequest {
    let mut system_parts = Vec::new();
    let mut contents = Vec::new();

    for message in messages {
        match message.role.as_str() {
            "system" => system_parts.push(GeminiPart {
                text: message.content,
            }),
            "assistant" => contents.push(GeminiContent {
                role: Some("model".to_string()),
                parts: vec![GeminiPart {
                    text: message.content,
                }],
            }),
            _ => contents.push(GeminiContent {
                role: Some("user".to_string()),
                parts: vec![GeminiPart {
                    text: message.content,
                }],
            }),
        }
    }

    GeminiGenerateContentRequest {
        system_instruction: (!system_parts.is_empty()).then_some(GeminiContent {
            role: None,
            parts: system_parts,
        }),
        contents,
    }
}

pub fn extract_gemini_sse_deltas_from_bytes(
    buffer: &mut Vec<u8>,
    chunk: &[u8],
) -> Result<SseParseResult, AiClientError> {
    buffer.extend_from_slice(chunk);

    let mut deltas = Vec::new();

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
        if let Some(delta) = extract_gemini_delta_content(data.trim_start()) {
            deltas.push(delta);
        }
    }

    Ok(SseParseResult {
        deltas,
        done: false,
    })
}

pub fn extract_gemini_delta_content(data: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(data).ok()?;
    let parts = value
        .get("candidates")?
        .get(0)?
        .get("content")?
        .get("parts")?
        .as_array()?;
    let text = parts
        .iter()
        .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
        .collect::<Vec<_>>()
        .join("");
    (!text.is_empty()).then_some(text)
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
    for (_, value) in &provider.headers {
        let value = value.trim();
        if !value.is_empty() {
            secrets.push(value.to_string());
        }
    }
    secrets
}

impl Default for GeminiClient {
    fn default() -> Self {
        Self::new()
    }
}
