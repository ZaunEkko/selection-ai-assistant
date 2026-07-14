use tauri::{AppHandle, Emitter, State, WebviewWindow};
use tokio::time::{timeout, Duration};

use crate::ai::action_classifier::AiAction;
use crate::ai::anthropic::AnthropicClient;
use crate::ai::gemini::GeminiClient;
use crate::ai::openai_compatible::{AiClientError, ChatMessage, OpenAiCompatibleClient};
use crate::app_state::AppState;
use crate::commands::access::require_webview_label;
use crate::config::{AiProviderConfig, AiProviderKind, ProviderUpdate};
use crate::types::PublicError;

const AI_STREAM_TIMEOUT: Duration = Duration::from_secs(60);

fn public_ai_error(code: &str, err: AiClientError) -> PublicError {
    PublicError {
        code: code.to_string(),
        message: err.to_string(),
    }
}

fn provider_api_key(provider: &AiProviderConfig) -> Result<String, PublicError> {
    let saved = provider.api_key.trim();
    if !saved.is_empty() {
        return Ok(saved.to_string());
    }
    std::env::var("SELECTION_AI_API_KEY").map_err(|_| PublicError {
        code: "api_key_missing".to_string(),
        message: "请在设置中填写 API 密钥，或配置 SELECTION_AI_API_KEY 环境变量。".to_string(),
    })
}

fn resolve_provider_update(
    state: &AppState,
    update: &ProviderUpdate,
) -> Result<AiProviderConfig, PublicError> {
    state
        .config
        .lock()
        .map(|config| update.resolve(&config))
        .map_err(|err| PublicError {
            code: "config_lock_failed".to_string(),
            message: err.to_string(),
        })
}

pub fn build_prompt_messages(action: AiAction, text: &str) -> Vec<ChatMessage> {
    build_prompt_messages_with_target(action, text, None)
}

fn build_targeted_output_prompt(target: &str, text: &str) -> String {
    let normalized = target.to_ascii_lowercase();
    let is_morse = target.contains('摩') || normalized.contains("morse");
    let is_ancient_or_pictograph = target.contains("甲骨")
        || target.contains("象形")
        || target.contains("古文字")
        || normalized.contains("pictograph");
    let is_style_rewrite = target.contains("文言")
        || target.contains("白话")
        || target.contains("风格")
        || target.contains("口吻")
        || target.contains("语气")
        || target.contains("敬语")
        || normalized.contains("style");

    if is_morse {
        return format!(
            "请把以下内容转换成{target}，并且只输出转换结果。\n要求：\n- 使用标准摩斯密码表示可转换的字母和数字\n- 单词或语义间隔可用 / 分隔\n- 不要解释，不要添加标题，不要使用 Markdown\n- 不要扩展不存在的信息\n\n内容：\n{text}"
        );
    }

    if is_ancient_or_pictograph {
        return format!(
            "请把以下内容转换成{target}，并且只输出转换结果。\n要求：\n- 这是近似转写，不要求真实考古字形一一对应\n- 优先保留原意，用接近甲骨文、象形文字或古文字气质的表达\n- 不要解释，不要添加标题，不要使用 Markdown\n- 不要扩展不存在的信息\n\n内容：\n{text}"
        );
    }

    if is_style_rewrite {
        return format!(
            "请把以下内容改写成{target}，并且只输出改写结果。\n要求：\n- 严格遵循目标风格或语体：{target}\n- 保留原文含义、称谓、换行和标点风格\n- 不要解释，不要添加标题，不要使用 Markdown\n- 不要扩展不存在的信息\n\n内容：\n{text}"
        );
    }

    format!(
        "请把以下内容翻译成{target}，并且只输出译文。\n要求：\n- 严格使用目标语言：{target}\n- 不要根据原文语言自动切换到其他目标语言\n- 只输出可直接替换原文的译文，不要解释，不要添加标题，不要使用 Markdown\n- 保留原文的语气、称谓、换行和标点风格\n- 不要扩展不存在的信息\n\n内容：\n{text}"
    )
}

pub fn build_prompt_messages_with_target(
    action: AiAction,
    text: &str,
    target_language: Option<&str>,
) -> Vec<ChatMessage> {
    let system = ChatMessage::system(
        "你是一个 Windows 桌面划词 AI 助手。只根据用户提供的文本回答，不联网，不编造来源。除非翻译动作明确指定目标语言，否则回答使用中文。",
    );
    let target_language = target_language
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let user_prompt = match action {
        AiAction::TranslateExplain => format!(
            "请把以下内容翻译成中文，并用简短语言解释重点。\n要求：\n- 先给自然中文翻译\n- 再给 2-4 条解释\n- 不要扩展不存在的信息\n\n内容：\n{text}"
        ),
        AiAction::TranslateOnly => match target_language {
            Some(target) => build_targeted_output_prompt(target, text),
            None => format!(
                "请把以下内容翻译成目标语言，并且只输出译文。\n要求：\n- 如果原文主要是中文，翻译成自然英文\n- 如果原文主要是英文或其他语言，翻译成自然中文\n- 只输出译文，不要解释，不要添加标题，不要使用 Markdown\n- 不要扩展不存在的信息\n\n内容：\n{text}"
            ),
        },
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
        AiAction::ExpandPrompt => format!(
            "请把以下原始提示词或需求描述扩写成更清晰、更结构化、更容易让 AI 执行的中文提示词。\n\
要求：\n\
- 保留用户原意，不擅自改变目标\n\
- 不添加原文没有依据的事实、数据或背景\n\
- 补齐可执行表达，包括目标、上下文、约束、输出格式和注意事项\n\
- 先给出「优化后提示词」\n\
- 再用 3-5 条列出「主要改进点」\n\
- 如果原始需求信息不足，在提示词中加入需要用户补充的占位项\n\n\
原始提示词或需求描述：\n{text}"
        ),
        AiAction::MenuFallback => format!(
            "请根据以下内容给出简洁解释。\n要求：\n- 用中文回答\n- 不要联网\n- 不要添加原文没有的信息\n\n内容：\n{text}"
        ),
    };

    vec![system, ChatMessage::user(user_prompt)]
}

pub fn build_follow_up_prompt_messages(
    original_text: &str,
    previous_answer: &str,
    question: &str,
) -> Vec<ChatMessage> {
    let system = ChatMessage::system(
        "你是一个 Windows 桌面划词 AI 助手。只根据用户提供的原始文本、上一轮回答和追问回答，不联网，不编造来源。回答使用中文。",
    );
    let user_prompt = format!(
        "请基于原始选中文本和上一轮回答，回答用户追问。\n\
要求：\n\
- 用中文回答\n\
- 优先延续上一轮回答的上下文\n\
- 如果追问超出原始文本和上一轮回答能支持的范围，请明确说明信息不足\n\
- 不要添加没有依据的事实、数据或来源\n\n\
原始选中文本：\n{original_text}\n\n\
上一轮回答：\n{previous_answer}\n\n\
用户追问：\n{question}"
    );

    vec![system, ChatMessage::user(user_prompt)]
}

async fn list_provider_models_for_kind(
    provider: &crate::config::AiProviderConfig,
    api_key: &str,
) -> Result<Vec<String>, AiClientError> {
    match provider.provider_kind {
        AiProviderKind::OpenAiCompatible => {
            OpenAiCompatibleClient::new()
                .list_models(provider, api_key)
                .await
        }
        AiProviderKind::Anthropic => AnthropicClient::new().list_models(provider, api_key).await,
        AiProviderKind::Gemini => GeminiClient::new().list_models(provider, api_key).await,
    }
}

async fn stream_provider_chat<F>(
    provider: &crate::config::AiProviderConfig,
    api_key: &str,
    messages: Vec<ChatMessage>,
    on_delta: F,
) -> Result<(), AiClientError>
where
    F: FnMut(String) + Send,
{
    match provider.provider_kind {
        AiProviderKind::OpenAiCompatible => {
            OpenAiCompatibleClient::new()
                .stream_chat(provider, api_key, messages, on_delta)
                .await
        }
        AiProviderKind::Anthropic => {
            AnthropicClient::new()
                .stream_chat(provider, api_key, messages, on_delta)
                .await
        }
        AiProviderKind::Gemini => {
            GeminiClient::new()
                .stream_chat(provider, api_key, messages, on_delta)
                .await
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAiActionRequest {
    pub request_id: String,
    pub action: AiAction,
    pub text: String,
    pub target_language: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAiFollowUpRequest {
    pub request_id: String,
    pub original_text: String,
    pub previous_answer: String,
    pub question: String,
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
    let request_id_for_stream = request_id.clone();

    let stream_result = timeout(
        AI_STREAM_TIMEOUT,
        stream_provider_chat(&provider, &api_key, messages, move |delta| {
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

fn default_provider_with_api_key(
    state: &AppState,
) -> Result<(crate::config::AiProviderConfig, String), PublicError> {
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
    Ok((provider, api_key))
}

#[tauri::command]
pub async fn run_ai_action(
    webview: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    request: RunAiActionRequest,
) -> Result<RunAiActionResponse, PublicError> {
    require_webview_label(&webview, &["ai-panel", "floating-button"])?;
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

    let (provider, api_key) = default_provider_with_api_key(&state)?;

    let request_id = request.request_id.trim().to_string();
    let target_language = request
        .target_language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let messages =
        build_prompt_messages_with_target(request.action, request.text.trim(), target_language);
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

#[tauri::command]
pub async fn run_ai_follow_up(
    webview: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    request: RunAiFollowUpRequest,
) -> Result<RunAiActionResponse, PublicError> {
    require_webview_label(&webview, &["ai-panel"])?;
    if request.request_id.trim().is_empty() {
        return Err(PublicError {
            code: "request_id_required".to_string(),
            message: "运行追问前需要 request id。".to_string(),
        });
    }
    if request.original_text.trim().is_empty() {
        return Err(PublicError {
            code: "selection_text_required".to_string(),
            message: "追问前需要原始选中文本。".to_string(),
        });
    }
    if request.previous_answer.trim().is_empty() {
        return Err(PublicError {
            code: "previous_answer_required".to_string(),
            message: "追问前需要先生成一次回答。".to_string(),
        });
    }
    if request.question.trim().is_empty() {
        return Err(PublicError {
            code: "follow_up_question_required".to_string(),
            message: "请输入追问内容。".to_string(),
        });
    }

    let (provider, api_key) = default_provider_with_api_key(&state)?;
    let request_id = request.request_id.trim().to_string();
    let messages = build_follow_up_prompt_messages(
        request.original_text.trim(),
        request.previous_answer.trim(),
        request.question.trim(),
    );
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
    pub model_list_available: bool,
}

#[tauri::command]
pub async fn list_provider_models(
    webview: WebviewWindow,
    state: State<'_, AppState>,
    provider: ProviderUpdate,
) -> Result<Vec<String>, PublicError> {
    require_webview_label(&webview, &["main"])?;
    list_provider_models_for_update(&state, provider).await
}

pub async fn list_provider_models_for_update(
    state: &AppState,
    provider: ProviderUpdate,
) -> Result<Vec<String>, PublicError> {
    let provider = resolve_provider_update(state, &provider)?;
    list_provider_models_for_provider(provider).await
}

pub async fn list_provider_models_for_provider(
    provider: AiProviderConfig,
) -> Result<Vec<String>, PublicError> {
    let api_key = provider_api_key(&provider)?;
    list_provider_models_for_kind(&provider, &api_key)
        .await
        .map_err(|err| public_ai_error("provider_model_list_failed", err))
}

#[tauri::command]
pub async fn test_provider_connection(
    webview: WebviewWindow,
    state: State<'_, AppState>,
    provider: ProviderUpdate,
) -> Result<TestProviderConnectionResponse, PublicError> {
    require_webview_label(&webview, &["main"])?;
    test_provider_connection_for_update(&state, provider).await
}

pub async fn test_provider_connection_for_update(
    state: &AppState,
    provider: ProviderUpdate,
) -> Result<TestProviderConnectionResponse, PublicError> {
    let provider = resolve_provider_update(state, &provider)?;
    test_provider_connection_for_provider(provider).await
}

pub async fn test_provider_connection_for_provider(
    provider: AiProviderConfig,
) -> Result<TestProviderConnectionResponse, PublicError> {
    let api_key = provider_api_key(&provider)?;

    match list_provider_models_for_kind(&provider, &api_key).await {
        Ok(models) => Ok(TestProviderConnectionResponse {
            success: true,
            model_count: models.len(),
            model_list_available: true,
        }),
        Err(model_list_error) => {
            if provider.model.trim().is_empty() {
                return Err(public_ai_error(
                    "provider_model_list_failed",
                    model_list_error,
                ));
            }

            stream_provider_chat(
                &provider,
                &api_key,
                vec![ChatMessage::user("请只回复 OK，用于连接测试。")],
                |_| {},
            )
            .await
            .map_err(|err| PublicError {
                code: "provider_connection_failed".to_string(),
                message: format!("模型列表接口不可用，聊天接口测试也失败：{err}"),
            })?;

            Ok(TestProviderConnectionResponse {
                success: true,
                model_count: 0,
                model_list_available: false,
            })
        }
    }
}
