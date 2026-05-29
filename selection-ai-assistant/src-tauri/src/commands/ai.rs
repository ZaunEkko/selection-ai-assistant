use tauri::{AppHandle, Emitter, State};

use crate::ai::action_classifier::AiAction;
use crate::ai::openai_compatible::{ChatMessage, OpenAiCompatibleClient};
use crate::app_state::AppState;
use crate::types::PublicError;

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

#[tauri::command]
pub async fn run_ai_action(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RunAiActionRequest,
) -> Result<RunAiActionResponse, PublicError> {
    if request.text.trim().is_empty() {
        return Err(PublicError {
            code: "selection_text_required".to_string(),
            message: "Selected text is required before running an AI action.".to_string(),
        });
    }

    if request.request_id.trim().is_empty() {
        return Err(PublicError {
            code: "request_id_required".to_string(),
            message: "Request id is required before running an AI action.".to_string(),
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
        message: "Configure an AI provider before running an action.".to_string(),
    })?;

    let provider = config
        .providers
        .iter()
        .find(|item| item.id == provider_id)
        .cloned()
        .ok_or_else(|| PublicError {
            code: "provider_missing".to_string(),
            message: "Default provider was not found.".to_string(),
        })?;

    let api_key = std::env::var("SELECTION_AI_API_KEY").map_err(|_| PublicError {
        code: "api_key_missing".to_string(),
        message: "Set SELECTION_AI_API_KEY before running an AI action.".to_string(),
    })?;

    let request_id = request.request_id.trim().to_string();
    let messages = build_prompt_messages(request.action, request.text.trim());
    let response = RunAiActionResponse {
        request_id: request_id.clone(),
    };

    tauri::async_runtime::spawn(async move {
        let client = OpenAiCompatibleClient::new();
        let request_id_for_stream = request_id.clone();
        let app_for_stream = app.clone();

        let stream_result = client
            .stream_chat(&provider, &api_key, messages, move |delta| {
                let _ = app_for_stream.emit(
                    "ai_stream_delta",
                    serde_json::json!({
                        "requestId": request_id_for_stream.clone(),
                        "delta": delta,
                    }),
                );
            })
            .await;

        if let Err(err) = stream_result {
            let _ = app.emit(
                "ai_stream_delta",
                serde_json::json!({
                    "requestId": request_id.clone(),
                    "delta": format!("AI request failed: {err}"),
                }),
            );
        }

        let _ = app.emit(
            "ai_stream_done",
            serde_json::json!({
                "requestId": request_id,
            }),
        );
    });

    Ok(response)
}
