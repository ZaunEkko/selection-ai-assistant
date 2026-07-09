import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  formatCommandError,
  getConfig,
  getLatestPanelContext,
  getPlatformCapabilities,
  listProviderModels,
  openPanelFromFloatingButton,
  runScreenshotTranslate,
  cancelScreenshotTranslate,
  saveProviderConfig,
  showScreenshotOverlay,
  startDragAiPanel,
  testProviderConnection,
  type AiProviderConfig,
} from '../api/tauri';

const { invokeMock, startDraggingMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  startDraggingMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ startDragging: startDraggingMock }),
}));

describe('Tauri API wrappers', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    startDraggingMock.mockReset();
  });

  it('opens the AI panel from the stored current selection when the floating button is clicked', async () => {
    invokeMock.mockResolvedValue({});

    await openPanelFromFloatingButton();

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('open_panel_for_current_selection');
  });

  it('gets the latest panel context for missed panel events', async () => {
    invokeMock.mockResolvedValue({
      selection: { text: 'selected text', sourceApp: 'chrome.exe', windowTitle: 'Browser' },
      action: 'summarize',
      autoRun: true,
    });

    await expect(getLatestPanelContext()).resolves.toEqual({
      selection: { text: 'selected text', sourceApp: 'chrome.exe', windowTitle: 'Browser' },
      action: 'summarize',
      autoRun: true,
    });
    expect(invokeMock).toHaveBeenCalledWith('get_latest_panel_context');
  });

  it('starts dragging the current AI panel window', async () => {
    startDraggingMock.mockResolvedValue(undefined);

    await startDragAiPanel();

    expect(startDraggingMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('invokes config commands with expected payloads', async () => {
    const provider: AiProviderConfig = {
      id: 'openai',
      name: 'OpenAI',
      baseUrl: 'https://api.openai.com/v1',
      model: 'gpt-test',
      providerKind: 'openAiCompatible',
      apiKey: 'dummy-api-key',
      apiKeyRef: 'credential://openai',
      headers: [],
    };
    invokeMock.mockResolvedValue({ providers: [provider] });

    await getConfig();
    await saveProviderConfig(provider);

    expect(invokeMock).toHaveBeenNthCalledWith(1, 'get_config');
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'save_provider_config', { provider });
  });

  it('invokes provider model and connection commands with provider payloads', async () => {
    const provider: AiProviderConfig = {
      id: 'openai',
      name: 'OpenAI',
      baseUrl: 'https://api.openai.com/v1',
      model: '',
      providerKind: 'openAiCompatible',
      apiKey: 'dummy-api-key',
      apiKeyRef: 'credential://openai',
      headers: [],
    };
    invokeMock.mockResolvedValueOnce(['gpt-test']).mockResolvedValueOnce({ success: true, modelCount: 1, modelListAvailable: true });

    await expect(listProviderModels(provider)).resolves.toEqual(['gpt-test']);
    await expect(testProviderConnection(provider)).resolves.toEqual({ success: true, modelCount: 1, modelListAvailable: true });

    expect(invokeMock).toHaveBeenNthCalledWith(1, 'list_provider_models', { provider });
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'test_provider_connection', { provider });
  });

  it('gets platform capabilities for platform-aware UI fallbacks', async () => {
    const capabilities = {
      platform: 'macos',
      automaticSelection: 'permissionRequired',
      globalInputMonitor: 'permissionRequired',
      selectionReader: 'unavailable',
      selectionAnchorReader: 'unavailable',
      clipboardFallback: 'unavailable',
      manualHotkey: 'unavailable',
      permissionCheck: 'permissionRequired',
      permissionNote: 'macOS backend 已预留',
    } as const;
    invokeMock.mockResolvedValueOnce(capabilities);

    await expect(getPlatformCapabilities()).resolves.toEqual(capabilities);

    expect(invokeMock).toHaveBeenCalledWith('get_platform_capabilities');
  });

  it('invokes screenshot translation commands with expected payloads', async () => {
    const position = { x: 120, y: 80 };
    const request = {
      requestId: 'screenshot-1',
      rect: { x: 10, y: 20, width: 180, height: 90 },
      viewportSize: { width: 800, height: 600 },
    };
    invokeMock.mockResolvedValueOnce(undefined).mockResolvedValueOnce({ requestId: 'screenshot-1' }).mockResolvedValueOnce(undefined);

    await showScreenshotOverlay(position);
    await expect(runScreenshotTranslate(request)).resolves.toEqual({ requestId: 'screenshot-1' });
    await cancelScreenshotTranslate();

    expect(invokeMock).toHaveBeenNthCalledWith(1, 'show_screenshot_overlay', { position });
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'run_screenshot_translate', { request });
    expect(invokeMock).toHaveBeenNthCalledWith(3, 'cancel_screenshot_translate');
  });

  it('formats common command errors in Chinese while keeping safe details', () => {
    expect(formatCommandError({ code: 'api_key_missing', message: 'Enter an API key in Settings.' })).toBe(
      '请在设置中填写 API 密钥，或配置 SELECTION_AI_API_KEY 环境变量。',
    );
    expect(formatCommandError({ code: 'provider_model_list_failed', message: 'request failed: HTTP 401' })).toBe(
      '服务商模型接口请求失败：request failed: HTTP 401',
    );
    expect(formatCommandError({ code: 'provider_stream_timeout', message: 'AI 服务商响应超时。' })).toBe(
      'AI 服务商响应超时，请稍后重试或检查服务商配置。',
    );
    expect(formatCommandError(new Error('network timeout'))).toBe('network timeout');
  });
});
