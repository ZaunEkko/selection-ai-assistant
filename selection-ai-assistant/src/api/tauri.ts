import { invoke } from '@tauri-apps/api/core';

export type AiProviderConfig = {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
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

export type UiAction = 'translateExplain' | 'explain' | 'summarize' | 'codeExplain' | 'errorExplain' | 'menuFallback';

export type Point = {
  x: number;
  y: number;
};

export function runAiAction(request: { requestId: string; action: UiAction; text: string }): Promise<{ requestId: string }> {
  return invoke<{ requestId: string }>('run_ai_action', { request });
}

export function hideAiPanel(): Promise<void> {
  return invoke<void>('hide_ai_panel');
}

export function showAiPanel(position: Point): Promise<void> {
  return invoke<void>('show_ai_panel', { position });
}

export function hideFloatingButton(): Promise<void> {
  return invoke<void>('hide_floating_button');
}

export async function openPanelFromFloatingButton(position: Point = { x: 200, y: 200 }): Promise<void> {
  await showAiPanel(position);
  await hideFloatingButton();
}

export function formatCommandError(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (err && typeof err === 'object' && 'message' in err) {
    const message = (err as { message?: unknown }).message;
    if (typeof message === 'string') return message;
  }
  return String(err);
}
