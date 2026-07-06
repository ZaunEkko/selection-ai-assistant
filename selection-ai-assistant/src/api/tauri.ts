import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

export type AiProviderKind = 'openAiCompatible' | 'anthropic' | 'gemini';

export type AiProviderConfig = {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  providerKind: AiProviderKind;
  apiKey: string;
  apiKeyRef: string;
  headers: Array<[string, string]>;
};

export type CloseButtonBehavior = 'ask' | 'minimizeToTray' | 'exitApp';

export type AppBehaviorConfig = {
  hotkey: string;
  startMinimizedToTray: boolean;
  closeButtonBehavior: CloseButtonBehavior;
};

export type AppConfig = {
  defaultProviderId: string | null;
  providers: AiProviderConfig[];
  hoverRadius: number;
  hoverDelayMs: number;
  candidateTimeoutMs: number;
  minDragDistance: number;
  hotkey: string;
  clipboardFallbackEnabled: boolean;
  showClipboardPrivacyWarningOnFirstUse: boolean;
  disableInElevatedWindows: boolean;
  manualHotkeyAlwaysEnabled: boolean;
  startMinimizedToTray: boolean;
  closeButtonBehavior: CloseButtonBehavior;
  disabledApps: string[];
};

export type PlatformId = 'windows' | 'macos' | 'linux' | 'unknown';

export type PlatformFeatureStatus = 'supported' | 'unsupported' | 'permissionRequired' | 'unavailable';

export type PlatformCapabilities = {
  platform: PlatformId;
  automaticSelection: PlatformFeatureStatus;
  globalInputMonitor: PlatformFeatureStatus;
  selectionReader: PlatformFeatureStatus;
  selectionAnchorReader: PlatformFeatureStatus;
  clipboardFallback: PlatformFeatureStatus;
  manualHotkey: PlatformFeatureStatus;
  permissionCheck: PlatformFeatureStatus;
  permissionNote: string | null;
};

export function getPlatformCapabilities(): Promise<PlatformCapabilities> {
  return invoke<PlatformCapabilities>('get_platform_capabilities');
}

export function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config');
}

export function saveProviderConfig(provider: AiProviderConfig): Promise<AppConfig> {
  return invoke<AppConfig>('save_provider_config', { provider });
}

export function setDefaultProvider(providerId: string): Promise<AppConfig> {
  return invoke<AppConfig>('set_default_provider', { providerId });
}

export function deleteProvider(providerId: string): Promise<AppConfig> {
  return invoke<AppConfig>('delete_provider', { providerId });
}

export function saveAppBehaviorConfig(preferences: AppBehaviorConfig): Promise<AppConfig> {
  return invoke<AppConfig>('save_app_behavior_config', { preferences });
}

export function confirmMainWindowClose(behavior: CloseButtonBehavior): Promise<AppConfig> {
  return invoke<AppConfig>('confirm_main_window_close', { behavior });
}

export function listProviderModels(provider: AiProviderConfig): Promise<string[]> {
  return invoke<string[]>('list_provider_models', { provider });
}

export type TestProviderConnectionResult = {
  success: boolean;
  modelCount: number;
  modelListAvailable: boolean;
};

export function testProviderConnection(provider: AiProviderConfig): Promise<TestProviderConnectionResult> {
  return invoke<TestProviderConnectionResult>('test_provider_connection', { provider });
}

export type UiAction =
  | 'translateExplain'
  | 'translateOnly'
  | 'explain'
  | 'summarize'
  | 'codeExplain'
  | 'errorExplain'
  | 'expandPrompt'
  | 'menuFallback';

export type Point = {
  x: number;
  y: number;
};

export type Rect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type PanelContext = {
  selection: {
    id?: string;
    text: string;
    sourceApp: string;
    windowTitle: string;
    anchorRect?: Rect | null;
    fallbackPoint?: Point;
    selectionRects?: Rect[];
    explicitAnchor?: Point | null;
  };
  action: UiAction;
  autoRun?: boolean;
};

export function runAiAction(request: { requestId: string; action: UiAction; text: string }): Promise<{ requestId: string }> {
  return invoke<{ requestId: string }>('run_ai_action', { request });
}

export function runAiFollowUp(request: {
  requestId: string;
  originalText: string;
  previousAnswer: string;
  question: string;
}): Promise<{ requestId: string }> {
  return invoke<{ requestId: string }>('run_ai_follow_up', { request });
}

export type SourceTextContext = {
  text: string;
};

export function getLatestPanelContext(): Promise<PanelContext | null> {
  return invoke<PanelContext | null>('get_latest_panel_context');
}

export function getLatestSourceTextContext(): Promise<SourceTextContext | null> {
  return invoke<SourceTextContext | null>('get_latest_source_text_context');
}

export function hideAiPanel(): Promise<void> {
  return invoke<void>('hide_ai_panel');
}

export function showSourceTextWindow(text: string): Promise<void> {
  return invoke<void>('show_source_text_window', { text });
}

export function hideSourceTextWindow(): Promise<void> {
  return invoke<void>('hide_source_text_window');
}

export function startDragAiPanel(): Promise<void> {
  return getCurrentWindow().startDragging();
}

export function startDragSourceTextWindow(): Promise<void> {
  return getCurrentWindow().startDragging();
}

export function startDragTranslateResultWindow(): Promise<void> {
  return getCurrentWindow().startDragging();
}

export type WindowResizeDirection =
  | 'East'
  | 'North'
  | 'NorthEast'
  | 'NorthWest'
  | 'South'
  | 'SouthEast'
  | 'SouthWest'
  | 'West';

export function startResizeTranslateResultWindow(direction: WindowResizeDirection = 'SouthEast'): Promise<void> {
  return getCurrentWindow().startResizeDragging(direction);
}

export function showAiPanel(position: Point): Promise<void> {
  return invoke<void>('show_ai_panel', { position });
}

export function hideFloatingButton(): Promise<void> {
  return invoke<void>('hide_floating_button');
}

export function showTranslateResult(
  position: Point,
  originalText: string,
  translatedText: string,
  selectionRects: Rect[] = [],
): Promise<void> {
  return invoke<void>('show_translate_result', { position, originalText, translatedText, selectionRects });
}

export function hideTranslateResult(): Promise<void> {
  return invoke<void>('hide_translate_result');
}

export function replaceSelectedText(text: string, selectionId?: string): Promise<void> {
  return invoke<void>('replace_selected_text', { text, selectionId });
}

export async function openPanelFromFloatingButton(): Promise<void> {
  await invoke('open_panel_for_current_selection');
}

export function formatCommandError(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (err && typeof err === 'object') {
    const commandError = err as { code?: unknown; message?: unknown };
    const message = typeof commandError.message === 'string' ? commandError.message : String(err);

    if (commandError.code === 'api_key_missing') {
      return '请在设置中填写 API 密钥，或配置 SELECTION_AI_API_KEY 环境变量。';
    }
    if (commandError.code === 'provider_model_list_failed') {
      return `服务商模型接口请求失败：${message}`;
    }
    if (commandError.code === 'provider_stream_failed') {
      return `AI 服务商请求失败：${message}`;
    }
    if (commandError.code === 'provider_stream_timeout') {
      return 'AI 服务商响应超时，请稍后重试或检查服务商配置。';
    }
    if (commandError.code === 'provider_missing') {
      return `请先配置默认 AI 服务商。${message ? ` ${message}` : ''}`.trim();
    }

    return message;
  }
  return String(err);
}

export function copyToClipboard(text: string): Promise<void> {
  return invoke<void>('copy_to_clipboard', { text });
}
