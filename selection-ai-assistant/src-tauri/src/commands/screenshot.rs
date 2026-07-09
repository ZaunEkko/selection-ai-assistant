use tauri::{AppHandle, Emitter, Manager, State};
use tokio::time::{sleep, timeout, Duration};

use crate::{
    ai::openai_compatible::{AiClientError, OpenAiCompatibleClient},
    app_state::AppState,
    config::{AiProviderConfig, AiProviderKind},
    floating_window::positioning::{ScreenBounds, WindowSize},
    types::{Point, PublicError, Rect},
};

const SCREENSHOT_OVERLAY_LABEL: &str = "screenshot-overlay";
const TRANSLATE_RESULT_LABEL: &str = "translate-result";
const SCREENSHOT_TRANSLATE_TIMEOUT: Duration = Duration::from_secs(90);
const SCREENSHOT_CAPTURE_SETTLE_DELAY: Duration = Duration::from_millis(160);
const MIN_SCREENSHOT_SELECTION_SIZE: f64 = 8.0;
const SCREENSHOT_TRANSLATE_RESULT_SIZE: WindowSize = WindowSize {
    width: 320.0,
    height: 180.0,
};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScreenshotTranslateRequest {
    pub request_id: String,
    pub rect: Rect,
    #[serde(default)]
    pub viewport_size: Option<ScreenshotViewportSize>,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotViewportSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScreenshotTranslateResponse {
    pub request_id: String,
}

#[tauri::command]
pub fn show_screenshot_overlay(app: AppHandle, position: Point) -> Result<(), PublicError> {
    show_screenshot_overlay_for_point(&app, position)
}

pub fn show_screenshot_overlay_for_point(
    app: &AppHandle,
    position: Point,
) -> Result<(), PublicError> {
    let window = app
        .get_webview_window(SCREENSHOT_OVERLAY_LABEL)
        .ok_or_else(|| command_error("screenshot_overlay_missing", "截图取景窗口未配置。"))?;
    let screen = screen_bounds_for_anchor(app, position)?;

    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: screen.x.round() as i32,
            y: screen.y.round() as i32,
        }))
        .map_err(|err| command_error("set_position_failed", err))?;
    window
        .set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: screen.width.round().max(1.0) as u32,
            height: screen.height.round().max(1.0) as u32,
        }))
        .map_err(|err| command_error("set_size_failed", err))?;
    window
        .show()
        .map_err(|err| command_error("show_failed", err))?;
    window
        .set_focus()
        .map_err(|err| command_error("focus_failed", err))?;
    Ok(())
}

#[tauri::command]
pub fn cancel_screenshot_translate(app: AppHandle) -> Result<(), PublicError> {
    hide_screenshot_overlay(&app)
}

#[tauri::command]
pub async fn run_screenshot_translate(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RunScreenshotTranslateRequest,
) -> Result<RunScreenshotTranslateResponse, PublicError> {
    let rect = normalized_rect(request.rect).ok_or_else(|| {
        command_error(
            "screenshot_rect_too_small",
            "截图区域太小，请拖出更大的识别范围。",
        )
    })?;
    let request_id = if request.request_id.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        request.request_id.trim().to_string()
    };

    let (provider, api_key) = default_provider_with_api_key(&state)?;
    if provider.provider_kind != AiProviderKind::OpenAiCompatible {
        return Err(command_error(
            "provider_vision_unsupported",
            "截图翻译当前先支持 OpenAI-compatible 视觉模型；请使用 OpenRouter/OpenAI-compatible 并选择支持图片输入的模型。",
        ));
    }

    let overlay = app
        .get_webview_window(SCREENSHOT_OVERLAY_LABEL)
        .ok_or_else(|| command_error("screenshot_overlay_missing", "截图取景窗口未配置。"))?;
    let screen_rect = physical_screen_rect_from_overlay(&overlay, rect, request.viewport_size)?;
    hide_screenshot_overlay(&app)?;
    hide_translate_result(&app)?;
    sleep(SCREENSHOT_CAPTURE_SETTLE_DELAY).await;

    let image_data_url = capture_screen_region_png_data_url(screen_rect)?;
    show_screenshot_translate_result(&app, screen_rect, "")?;

    let response = RunScreenshotTranslateResponse {
        request_id: request_id.clone(),
    };
    let app_for_stream = app.clone();
    tauri::async_runtime::spawn(async move {
        stream_screenshot_translate(
            app_for_stream,
            provider,
            api_key,
            request_id,
            image_data_url,
        )
        .await;
    });

    Ok(response)
}

async fn stream_screenshot_translate(
    app: AppHandle,
    provider: AiProviderConfig,
    api_key: String,
    request_id: String,
    image_data_url: String,
) {
    let system_prompt = "你是一个 Windows 桌面截图翻译助手。只根据用户提供的截图内容进行 OCR 和翻译，不联网，不编造来源。";
    let user_prompt = "请识别截图中的可见文字，并翻译成自然中文。要求：\n- 如果截图里没有可读文字，只回答“未识别到可翻译文字”。\n- 先输出译文，不要添加标题。\n- 对很短的 UI 文案保持简洁。\n- 不要解释截图以外的信息。";
    let stream_result = timeout(
        SCREENSHOT_TRANSLATE_TIMEOUT,
        OpenAiCompatibleClient::new().stream_vision_chat(
            &provider,
            &api_key,
            system_prompt,
            user_prompt,
            image_data_url,
            |delta| {
                let _ = app.emit(
                    "translate_result_delta",
                    serde_json::json!({
                        "delta": delta,
                    }),
                );
            },
        ),
    )
    .await;

    let error_message = match stream_result {
        Ok(Ok(())) => None,
        Ok(Err(err)) => Some(provider_error_message(err)),
        Err(_) => Some("截图翻译超时，请稍后重试或缩小截图区域。".to_string()),
    };

    if let Some(message) = error_message {
        let _ = app.emit(
            "translate_result_delta",
            serde_json::json!({
                "delta": format!("\n\n截图翻译失败：{message}"),
            }),
        );
    }

    let _ = app.emit(
        "screenshot_translate_done",
        serde_json::json!({
            "requestId": request_id,
        }),
    );
}

fn show_screenshot_translate_result(
    app: &AppHandle,
    screen_rect: Rect,
    translated_text: &str,
) -> Result<(), PublicError> {
    let window = app
        .get_webview_window(TRANSLATE_RESULT_LABEL)
        .ok_or_else(|| {
            command_error(
                "translate_result_missing",
                "Translate result window is not configured.",
            )
        })?;
    let position = translate_result_position(app, screen_rect, &window)?;

    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: position.x.round() as i32,
            y: position.y.round() as i32,
        }))
        .map_err(|err| command_error("set_position_failed", err))?;
    window
        .show()
        .map_err(|err| command_error("show_failed", err))?;
    window
        .emit(
            "translate_result",
            crate::commands::panel::TranslateResultPayload {
                original_text: "截图区域".to_string(),
                translated_text: translated_text.to_string(),
            },
        )
        .map_err(|err| command_error("emit_failed", err))?;
    Ok(())
}

fn translate_result_position(
    app: &AppHandle,
    screen_rect: Rect,
    window: &tauri::WebviewWindow,
) -> Result<Point, PublicError> {
    let screen = screen_bounds_for_anchor(app, screen_rect.center())?;
    let size = window
        .outer_size()
        .map(|size| WindowSize {
            width: size.width as f64,
            height: size.height as f64,
        })
        .unwrap_or(SCREENSHOT_TRANSLATE_RESULT_SIZE);
    Ok(
        crate::floating_window::positioning::place_translate_result_near_selection(
            screen_rect.center(),
            &[screen_rect],
            size,
            screen,
        ),
    )
}

fn physical_screen_rect_from_overlay(
    overlay: &tauri::WebviewWindow,
    css_rect: Rect,
    viewport_size: Option<ScreenshotViewportSize>,
) -> Result<Rect, PublicError> {
    let overlay_position = overlay
        .outer_position()
        .map_err(|err| command_error("overlay_position_unavailable", err))?;
    let overlay_size = overlay
        .outer_size()
        .map_err(|err| command_error("overlay_size_unavailable", err))?;
    let scale_x = viewport_size
        .filter(|size| size.width > 0.0 && size.height > 0.0)
        .map(|size| overlay_size.width as f64 / size.width)
        .unwrap_or(1.0);
    let scale_y = viewport_size
        .filter(|size| size.width > 0.0 && size.height > 0.0)
        .map(|size| overlay_size.height as f64 / size.height)
        .unwrap_or(1.0);

    Ok(Rect {
        x: overlay_position.x as f64 + css_rect.x * scale_x,
        y: overlay_position.y as f64 + css_rect.y * scale_y,
        width: css_rect.width * scale_x,
        height: css_rect.height * scale_y,
    })
}

fn hide_translate_result(app: &AppHandle) -> Result<(), PublicError> {
    if let Some(window) = app.get_webview_window(TRANSLATE_RESULT_LABEL) {
        window
            .hide()
            .map_err(|err| command_error("hide_failed", err))?;
    }
    Ok(())
}

fn hide_screenshot_overlay(app: &AppHandle) -> Result<(), PublicError> {
    if let Some(window) = app.get_webview_window(SCREENSHOT_OVERLAY_LABEL) {
        window
            .hide()
            .map_err(|err| command_error("hide_failed", err))?;
    }
    Ok(())
}

fn default_provider_with_api_key(
    state: &AppState,
) -> Result<(crate::config::AiProviderConfig, String), PublicError> {
    let config = state
        .config
        .lock()
        .map_err(|err| command_error("config_lock_failed", err))?
        .clone();

    let provider_id = config
        .default_provider_id
        .as_deref()
        .ok_or_else(|| command_error("provider_missing", "截图翻译前需要先配置默认 AI 服务商。"))?;

    let provider = config
        .providers
        .iter()
        .find(|item| item.id == provider_id)
        .cloned()
        .ok_or_else(|| command_error("provider_missing", "未找到默认服务商配置。"))?;
    let saved_key = provider.api_key.trim();
    let api_key = if saved_key.is_empty() {
        std::env::var("SELECTION_AI_API_KEY").map_err(|_| {
            command_error(
                "api_key_missing",
                "请在设置中填写 API 密钥，或配置 SELECTION_AI_API_KEY 环境变量。",
            )
        })?
    } else {
        saved_key.to_string()
    };

    Ok((provider, api_key))
}

fn normalized_rect(rect: Rect) -> Option<Rect> {
    let left = rect.x.min(rect.x + rect.width);
    let top = rect.y.min(rect.y + rect.height);
    let width = rect.width.abs();
    let height = rect.height.abs();

    if width < MIN_SCREENSHOT_SELECTION_SIZE || height < MIN_SCREENSHOT_SELECTION_SIZE {
        return None;
    }

    Some(Rect {
        x: left,
        y: top,
        width,
        height,
    })
}

fn screen_bounds_for_anchor(app: &AppHandle, anchor: Point) -> Result<ScreenBounds, PublicError> {
    let monitors = app
        .available_monitors()
        .map_err(|err| command_error("monitor_unavailable", err))?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            let x = position.x as f64;
            let y = position.y as f64;
            anchor.x >= x
                && anchor.x <= x + size.width as f64
                && anchor.y >= y
                && anchor.y <= y + size.height as f64
        })
        .or_else(|| monitors.first())
        .ok_or_else(|| {
            command_error(
                "monitor_unavailable",
                "No monitor is available for screenshot overlay.",
            )
        })?;

    let position = monitor.position();
    let size = monitor.size();
    Ok(ScreenBounds {
        x: position.x as f64,
        y: position.y as f64,
        width: size.width as f64,
        height: size.height as f64,
    })
}

fn capture_screen_region_png_data_url(rect: Rect) -> Result<String, PublicError> {
    #[cfg(windows)]
    {
        crate::platform::windows::capture_screen_region_png_data_url(rect)
    }

    #[cfg(not(windows))]
    {
        let _ = rect;
        Err(command_error(
            "screenshot_unavailable",
            "当前平台暂未实现截图翻译。",
        ))
    }
}

fn provider_error_message(err: AiClientError) -> String {
    err.to_string()
}

fn command_error(code: &str, err: impl ToString) -> PublicError {
    PublicError {
        code: code.to_string(),
        message: err.to_string(),
    }
}
