import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

export type AiProviderConfig = {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  apiKey: string;
  apiKeyRef: string;
  headers: Array<[string, string]>;
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
  disabledApps: string[];
};

export function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config');
}

export function saveProviderConfig(provider: AiProviderConfig): Promise<AppConfig> {
  return invoke<AppConfig>('save_provider_config', { provider });
}

export function listProviderModels(provider: AiProviderConfig): Promise<string[]> {
  return invoke<string[]>('list_provider_models', { provider });
}

export function testProviderConnection(provider: AiProviderConfig): Promise<{ success: boolean; modelCount: number }> {
  return invoke<{ success: boolean; modelCount: number }>('test_provider_connection', { provider });
}

export type UiAction = 'translateExplain' | 'explain' | 'summarize' | 'codeExplain' | 'errorExplain' | 'menuFallback';

export type PanelContext = {
  selection: {
    id?: string;
    text: string;
    sourceApp: string;
    windowTitle: string;
  };
  action: UiAction;
  autoRun?: boolean;
};

export type Point = {
  x: number;
  y: number;
};

export function runAiAction(request: { requestId: string; action: UiAction; text: string }): Promise<{ requestId: string }> {
  return invoke<{ requestId: string }>('run_ai_action', { request });
}

export function getLatestPanelContext(): Promise<PanelContext | null> {
  return invoke<PanelContext | null>('get_latest_panel_context');
}

export function hideAiPanel(): Promise<void> {
  return invoke<void>('hide_ai_panel');
}

export function startDragAiPanel(): Promise<void> {
  return getCurrentWindow().startDragging();
}

export function showAiPanel(position: Point): Promise<void> {
  return invoke<void>('show_ai_panel', { position });
}

export function hideFloatingButton(): Promise<void> {
  return invoke<void>('hide_floating_button');
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
