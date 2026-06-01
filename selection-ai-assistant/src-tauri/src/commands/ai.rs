use tauri::{AppHandle, Emitter, State};
use tokio::time::{timeout, Duration};

use crate::ai::action_classifier::AiAction;
use crate::ai::openai_compatible::{AiClientError, ChatMessage, OpenAiCompatibleClient};
use crate::app_state::AppState;
use crate::types::PublicError;

const AI_STREAM_TIMEOUT: Duration = Duration::from_secs(60);

fn public_ai_error(code: &str, err: AiClientError) -> PublicError {
    PublicError {
        code: code.to_string(),
        message: err.to_string(),
    }
}

fn provider_api_key(provider: &crate::config::AiProviderConfig) -> Result<String, PublicError> {
    let saved = provider.api_key.trim();
    if !saved.is_empty() {
        return Ok(saved.to_string());
    }
    std::env::var("SELECTION_AI_API_KEY").map_err(|_| PublicError {
        code: "api_key_missing".to_string(),
        message: "请在设置中填写 API 密钥，或配置 SELECTION_AI_API_KEY 环境变量。".to_string(),
    })
}

pub fn build_prompt_messages(action: AiAction, text: &str) -> Vec<ChatMessage> {
    let system = ChatMessage::system(
        "你是一个 Windows 桌面划词 AI 助手。只根据用户提供的文本回答，不联网，不编造来源。回答使用中文。",
    );

    let user_prompt = match action {
        AiAction::TranslateExplain => format!(
            "请把以下内容翻译成中文，并用简短语言解释重点。\n要求：\n- 先给自然中文翻译\n- 再给 2-4 条解释\n- 不要扩展不存在的信息\n\n内容：\n{text}"
        ),
        AiAction::Explain => format!(
            "请解释以下内容。\n要求：\n- 用中文回答\n- 先给一句话概括\n- 再解释关键概念\n- 如果内容可能有歧义，指出歧义\n- 不要联网，不要编造来源\n\n内容：\n{text}"
        ),
        AiAction::Summarize => format!(
            "请总结以下内容。\n要求：\n- 先给 3-5 条要点\n- 再给一句结论\n- 保留关键数字、名称和条件\n- 不要添加原文没有的信息\n\n内容：\n{text}"
        ),
        AiAction::CodeExplain => format!(
            "请解释以下代码。\n要求：\n- 判断语言或技术栈\n- 说明这段代码在做什么\n- 标出关键逻辑\n- 如有明显问题，指出可能风险\n- 不要重写整段代码，除非我追问\n\n代码：\n{text}"
        ),
        AiAction::ErrorExplain => format!(
            "请解释以下报错。\n要求：\n- 判断可能原因\n- 给出排查步骤\n- 给出可能修复方向\n- 如果信息不足，说明还需要哪些上下文\n\n报错：\n{text}"
        ),
        AiAction::MenuFallback => format!(
            "请根据以下内容给出简洁解释。\n要求：\n- 用中文回答\n- 不要联网\n- 不要添加原文没有的信息\n\n内容：\n{text}"
        ),
    };

    vec![system, ChatMessage::user(user_prompt)]
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAiActionRequest {
    pub request_id: String,
    pub action: AiAction,
    pub text: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAiActionResponse {
    pub request_id: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStreamErrorPayload {
    pub request_id: String,
    pub code: String,
    pub message: String,
}

pub async fn stream_chat_events_for_request<OnDelta, OnError, OnDone>(
    provider: crate::config::AiProviderConfig,
    api_key: String,
    request_id: String,
    messages: Vec<ChatMessage>,
    mut on_delta: OnDelta,
    mut on_error: OnError,
    mut on_done: OnDone,
) where
    OnDelta: FnMut(String, String) + Send,
    OnError: FnMut(AiStreamErrorPayload) + Send,
    OnDone: FnMut(String) + Send,
{
    let client = OpenAiCompatibleClient::new();
    let request_id_for_stream = request_id.clone();

    let stream_result = timeout(
        AI_STREAM_TIMEOUT,
        client.stream_chat(&provider, &api_key, messages, move |delta| {
            on_delta(request_id_for_stream.clone(), delta);
        }),
    )
    .await;

    match stream_result {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            on_error(AiStreamErrorPayload {
                request_id: request_id.clone(),
                code: "provider_stream_failed".to_string(),
                message: err.to_string(),
            });
        }
        Err(_) => {
            on_error(AiStreamErrorPayload {
                request_id: request_id.clone(),
                code: "provider_stream_timeout".to_string(),
                message: "AI 服务商响应超时。".to_string(),
            });
        }
    }

    on_done(request_id);
}

#[tauri::command]
pub async fn run_ai_action(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RunAiActionRequest,
) -> Result<RunAiActionResponse, PublicError> {
    if request.text.trim().is_empty() {
        return Err(PublicError {
            code: "selection_text_required".to_string(),
            message: "运行 AI 动作前需要选中文本。".to_string(),
        });
    }

    if request.request_id.trim().is_empty() {
        return Err(PublicError {
            code: "request_id_required".to_string(),
            message: "运行 AI 动作前需要 request id。".to_string(),
        });
    }

    let config = state
        .config
        .lock()
        .map_err(|err| PublicError {
            code: "config_lock_failed".to_string(),
            message: err.to_string(),
        })?
        .clone();

    let provider_id = config.default_provider_id.ok_or_else(|| PublicError {
        code: "provider_missing".to_string(),
        message: "运行 AI 动作前需要先配置默认服务商。".to_string(),
    })?;

    let provider = config
        .providers
        .iter()
        .find(|item| item.id == provider_id)
        .cloned()
        .ok_or_else(|| PublicError {
            code: "provider_missing".to_string(),
            message: "未找到默认服务商配置。".to_string(),
        })?;

    let api_key = provider_api_key(&provider)?;

    let request_id = request.request_id.trim().to_string();
    let messages = build_prompt_messages(request.action, request.text.trim());
    let response = RunAiActionResponse {
        request_id: request_id.clone(),
    };

    tauri::async_runtime::spawn(async move {
        stream_chat_events_for_request(
            provider,
            api_key,
            request_id,
            messages,
            |request_id, delta| {
                let _ = app.emit(
                    "ai_stream_delta",
                    serde_json::json!({
                        "requestId": request_id,
                        "delta": delta,
                    }),
                );
            },
            |payload| {
                let _ = app.emit("ai_stream_error", payload);
            },
            |request_id| {
                let _ = app.emit(
                    "ai_stream_done",
                    serde_json::json!({
                        "requestId": request_id,
                    }),
                );
            },
        )
        .await;
    });

    Ok(response)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestProviderConnectionResponse {
    pub success: bool,
    pub model_count: usize,
}

#[tauri::command]
pub async fn list_provider_models(
    provider: crate::config::AiProviderConfig,
) -> Result<Vec<String>, PublicError> {
    let api_key = provider_api_key(&provider)?;
    OpenAiCompatibleClient::new()
        .list_models(&provider, &api_key)
        .await
        .map_err(|err| public_ai_error("provider_model_list_failed", err))
}

#[tauri::command]
pub async fn test_provider_connection(
    provider: crate::config::AiProviderConfig,
) -> Result<TestProviderConnectionResponse, PublicError> {
    let models = list_provider_models(provider).await?;
    Ok(TestProviderConnectionResponse {
        success: true,
        model_count: models.len(),
    })
}
