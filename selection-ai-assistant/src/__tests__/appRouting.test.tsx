import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from '../App';

let currentLabel = 'main';
type Listener<T = unknown> = (event: { payload: T }) => void;

const {
  listeners,
  focusListeners,
  openPanelFromFloatingButtonMock,
  runAiActionMock,
  saveAppBehaviorConfigMock,
  showReplacementPresetPanelMock,
  hideReplacementPresetPanelMock,
  showTranslateResultMock,
  hideTranslateResultMock,
  hideFloatingButtonMock,
  replaceSelectedTextMock,
  hideSourceTextWindowMock,
  getLatestPanelContextMock,
  getLatestSourceTextContextMock,
  currentWindowHideMock,
  currentWindowStartDraggingMock,
  currentWindowStartResizeDraggingMock,
} = vi.hoisted(() => ({
  listeners: new Map<string, Listener[]>(),
  focusListeners: [] as Listener<boolean>[],
  openPanelFromFloatingButtonMock: vi.fn(),
  runAiActionMock: vi.fn(),
  saveAppBehaviorConfigMock: vi.fn(),
  showReplacementPresetPanelMock: vi.fn(),
  hideReplacementPresetPanelMock: vi.fn(),
  showTranslateResultMock: vi.fn(),
  hideTranslateResultMock: vi.fn(),
  hideFloatingButtonMock: vi.fn(),
  replaceSelectedTextMock: vi.fn(),
  hideSourceTextWindowMock: vi.fn(),
  getLatestPanelContextMock: vi.fn(),
  getLatestSourceTextContextMock: vi.fn(),
  currentWindowHideMock: vi.fn(),
  currentWindowStartDraggingMock: vi.fn(),
  currentWindowStartResizeDraggingMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    label: currentLabel,
    startDragging: currentWindowStartDraggingMock,
    startResizeDragging: currentWindowStartResizeDraggingMock,
    hide: currentWindowHideMock,
    onFocusChanged: (callback: Listener<boolean>) => {
      focusListeners.push(callback);
      return Promise.resolve(() => {
        const index = focusListeners.indexOf(callback);
        if (index >= 0) focusListeners.splice(index, 1);
      });
    },
  }),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (eventName: string, callback: Listener) => {
    const existing = listeners.get(eventName) ?? [];
    existing.push(callback);
    listeners.set(eventName, existing);
    return Promise.resolve(() => {
      listeners.set(
        eventName,
        (listeners.get(eventName) ?? []).filter((item) => item !== callback),
      );
    });
  },
  emit: vi.fn(() => Promise.resolve()),
}));

vi.mock('../api/tauri', () => ({
  getConfig: vi.fn(() =>
    Promise.resolve({
      defaultProviderId: null,
      providers: [],
      hoverRadius: 90,
      hoverDelayMs: 220,
      candidateTimeoutMs: 4000,
      minDragDistance: 6,
      hotkey: 'Ctrl+Alt+A',
      clipboardFallbackEnabled: true,
      showClipboardPrivacyWarningOnFirstUse: true,
      disableInElevatedWindows: true,
      manualHotkeyAlwaysEnabled: true,
      startMinimizedToTray: false,
      closeButtonBehavior: 'ask',
      replacementTargetLanguage: 'korean',
      replacementCustomTarget: '',
      disabledApps: [],
    }),
  ),
  saveProviderConfig: vi.fn(),
  saveAppBehaviorConfig: saveAppBehaviorConfigMock,
  getPlatformCapabilities: vi.fn(() =>
    Promise.resolve({
      platform: 'windows',
      automaticSelection: 'supported',
      globalInputMonitor: 'supported',
      selectionReader: 'supported',
      selectionAnchorReader: 'supported',
      clipboardFallback: 'supported',
      manualHotkey: 'supported',
      permissionCheck: 'supported',
      permissionNote: null,
    }),
  ),
  hideAiPanel: vi.fn(),
  hideSourceTextWindow: hideSourceTextWindowMock,
  showSourceTextWindow: vi.fn(),
  startDragAiPanel: vi.fn(),
  startDragSourceTextWindow: vi.fn(),
  startDragTranslateResultWindow: currentWindowStartDraggingMock,
  startResizeTranslateResultWindow: currentWindowStartResizeDraggingMock,
  runAiAction: runAiActionMock,
  runAiFollowUp: vi.fn(),
  getLatestPanelContext: getLatestPanelContextMock,
  getLatestSourceTextContext: getLatestSourceTextContextMock,
  openPanelFromFloatingButton: openPanelFromFloatingButtonMock,
  showReplacementPresetPanel: showReplacementPresetPanelMock,
  hideReplacementPresetPanel: hideReplacementPresetPanelMock,
  showTranslateResult: showTranslateResultMock,
  hideTranslateResult: hideTranslateResultMock,
  hideFloatingButton: hideFloatingButtonMock,
  replaceSelectedText: replaceSelectedTextMock,
  formatCommandError: (err: unknown) => (err instanceof Error ? err.message : String(err)),
}));

function emit<T>(eventName: string, payload: T) {
  for (const listener of listeners.get(eventName) ?? []) {
    listener({ payload });
  }
}

function emitFocusChanged(focused: boolean) {
  for (const listener of focusListeners) {
    listener({ payload: focused });
  }
}

describe('App routing by Tauri window label', () => {
  beforeEach(() => {
    currentLabel = 'main';
    listeners.clear();
    focusListeners.splice(0);
    runAiActionMock.mockReset();
    saveAppBehaviorConfigMock.mockReset();
    showTranslateResultMock.mockReset();
    showReplacementPresetPanelMock.mockReset();
    hideReplacementPresetPanelMock.mockReset();
    hideFloatingButtonMock.mockReset();
    replaceSelectedTextMock.mockReset();
    openPanelFromFloatingButtonMock.mockReset();
    hideSourceTextWindowMock.mockReset();
    hideTranslateResultMock.mockReset();
    getLatestPanelContextMock.mockReset();
    getLatestSourceTextContextMock.mockReset();
    currentWindowHideMock.mockReset();
    currentWindowStartDraggingMock.mockReset();
    currentWindowStartResizeDraggingMock.mockReset();
    runAiActionMock.mockResolvedValue({ requestId: 'request-1' });
    saveAppBehaviorConfigMock.mockResolvedValue({
      defaultProviderId: null,
      providers: [],
      hoverRadius: 90,
      hoverDelayMs: 220,
      candidateTimeoutMs: 4000,
      minDragDistance: 6,
      hotkey: 'Ctrl+Alt+A',
      clipboardFallbackEnabled: true,
      showClipboardPrivacyWarningOnFirstUse: true,
      disableInElevatedWindows: true,
      manualHotkeyAlwaysEnabled: true,
      startMinimizedToTray: false,
      closeButtonBehavior: 'ask',
      replacementTargetLanguage: 'korean',
      replacementCustomTarget: '',
      disabledApps: [],
    });
    showTranslateResultMock.mockResolvedValue(undefined);
    showReplacementPresetPanelMock.mockResolvedValue(undefined);
    hideReplacementPresetPanelMock.mockResolvedValue(undefined);
    hideTranslateResultMock.mockResolvedValue(undefined);
    hideFloatingButtonMock.mockResolvedValue(undefined);
    replaceSelectedTextMock.mockResolvedValue(undefined);
    openPanelFromFloatingButtonMock.mockResolvedValue(undefined);
    hideSourceTextWindowMock.mockResolvedValue(undefined);
    getLatestPanelContextMock.mockResolvedValue(null);
    getLatestSourceTextContextMock.mockResolvedValue(null);
  });

  it('renders mini action bar for the floating-button window without requiring window position APIs', () => {
    currentLabel = 'floating-button';

    render(<App />);

    expect(screen.getByRole('toolbar', { name: '文本操作' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '翻译并替换文本' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '翻译文本' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '更多操作' })).toBeInTheDocument();
  });

  it('renders mini action bar inside a transparent window root container', () => {
    currentLabel = 'floating-button';

    render(<App />);

    const toolbar = screen.getByRole('toolbar', { name: '文本操作' });
    expect(toolbar).toHaveClass('mini-action-bar');
    expect(toolbar.closest('.mini-action-bar-window')).not.toBeNull();
  });

  it('renders mini action bar without image assets', () => {
    currentLabel = 'floating-button';

    render(<App />);

    const toolbar = screen.getByRole('toolbar', { name: '文本操作' });
    expect(toolbar.querySelector('img')).toBeNull();
  });

  it('opens the replacement preset panel when the replace button is hovered or receives focus', async () => {
    currentLabel = 'floating-button';

    render(<App />);
    const replaceButton = screen.getByRole('button', { name: '翻译并替换文本' });
    fireEvent.mouseEnter(replaceButton);

    await waitFor(() => expect(showReplacementPresetPanelMock).toHaveBeenCalledTimes(1));

    fireEvent.focus(replaceButton);

    await waitFor(() => expect(showReplacementPresetPanelMock).toHaveBeenCalledTimes(2));
  });

  it('opens the full panel when more actions is clicked', async () => {
    currentLabel = 'floating-button';

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: '更多操作' }));

    await waitFor(() => expect(openPanelFromFloatingButtonMock).toHaveBeenCalledTimes(1));
  });

  it('shows translate result when the translation stream completes immediately after request starts', async () => {
    currentLabel = 'floating-button';
    getLatestPanelContextMock.mockResolvedValue({
      selection: {
        text: 'hello world',
        sourceApp: 'manual',
        windowTitle: 'Manual hotkey',
        fallbackPoint: { x: 320, y: 240 },
      },
      action: 'translateExplain',
    });
    runAiActionMock.mockImplementation(async (request: { requestId: string }) => {
      emit('ai_stream_delta', { requestId: request.requestId, delta: '你好世界' });
      emit('ai_stream_done', { requestId: request.requestId });
      return { requestId: request.requestId };
    });

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: '翻译文本' }));

    await waitFor(() =>
      expect(showTranslateResultMock).toHaveBeenNthCalledWith(1, { x: 320, y: 240 }, 'hello world', '', []),
    );
    await waitFor(() => expect(showTranslateResultMock).toHaveBeenLastCalledWith({ x: 320, y: 240 }, 'hello world', '你好世界', []));
    expect(hideReplacementPresetPanelMock).toHaveBeenCalledTimes(1);
    expect(runAiActionMock).toHaveBeenCalledWith(
      expect.objectContaining({ action: 'translateOnly', text: 'hello world', requestId: expect.stringMatching(/^translate-/) }),
    );
  });

  it('opens translate result immediately while the translation stream is still pending', async () => {
    currentLabel = 'floating-button';
    getLatestPanelContextMock.mockResolvedValue({
      selection: {
        text: 'hello world',
        sourceApp: 'manual',
        windowTitle: 'Manual hotkey',
        fallbackPoint: { x: 320, y: 240 },
      },
      action: 'translateExplain',
    });
    runAiActionMock.mockResolvedValue({ requestId: 'pending-request' });

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: '翻译文本' }));

    await waitFor(() =>
      expect(showTranslateResultMock).toHaveBeenCalledWith({ x: 320, y: 240 }, 'hello world', '', []),
    );
    expect(screen.getByRole('button', { name: '翻译文本' })).toHaveTextContent('翻译中…');
  });

  it('passes selection geometry to translate result placement', async () => {
    currentLabel = 'floating-button';
    getLatestPanelContextMock.mockResolvedValue({
      selection: {
        text: 'hello world',
        sourceApp: 'manual',
        windowTitle: 'Manual hotkey',
        selectionRects: [{ x: 100, y: 80, width: 160, height: 22 }],
        fallbackPoint: { x: 320, y: 240 },
      },
      action: 'translateExplain',
    });
    runAiActionMock.mockResolvedValue({ requestId: 'pending-request' });

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: '翻译文本' }));

    await waitFor(() =>
      expect(showTranslateResultMock).toHaveBeenCalledWith(
        { x: 100, y: 80 },
        'hello world',
        '',
        [{ x: 100, y: 80, width: 160, height: 22 }],
      ),
    );
  });

  it('ignores text-space selection geometry when opening translate result', async () => {
    currentLabel = 'floating-button';
    getLatestPanelContextMock.mockResolvedValue({
      selection: {
        text: 'hello world',
        sourceApp: 'manual',
        windowTitle: 'Manual hotkey',
        selectionRects: [{ x: 0, y: 60, width: 1800, height: 120 }],
        explicitAnchor: { x: 900, y: 120 },
        fallbackPoint: { x: 420, y: 88 },
      },
      action: 'translateExplain',
    });
    runAiActionMock.mockResolvedValue({ requestId: 'pending-request' });

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: '翻译文本' }));

    await waitFor(() =>
      expect(showTranslateResultMock).toHaveBeenCalledWith(
        { x: 420, y: 88 },
        'hello world',
        '',
        [],
      ),
    );
  });

  it('replaces the selected text with a translate-only stream when the replace stream completes immediately after request starts', async () => {
    currentLabel = 'floating-button';
    getLatestPanelContextMock.mockResolvedValue({
      selection: {
        id: 'selection-replace',
        text: '你好世界',
        sourceApp: 'manual',
        windowTitle: 'Manual hotkey',
        fallbackPoint: { x: 320, y: 240 },
      },
      action: 'translateExplain',
    });
    runAiActionMock.mockImplementation(async (request: { requestId: string }) => {
      emit('ai_stream_delta', { requestId: request.requestId, delta: 'Hello world' });
      emit('ai_stream_done', { requestId: request.requestId });
      return { requestId: request.requestId };
    });

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: '翻译并替换文本' }));

    await waitFor(() => expect(replaceSelectedTextMock).toHaveBeenCalledWith('Hello world', 'selection-replace'));
    expect(hideReplacementPresetPanelMock).toHaveBeenCalledTimes(1);
    expect(runAiActionMock).toHaveBeenCalledWith(
      expect.objectContaining({
        action: 'translateOnly',
        text: '你好世界',
        targetLanguage: '韩文',
        requestId: expect.stringMatching(/^replace-/),
      }),
    );
  });

  it('shows replace progress text while the replacement stream is pending', async () => {
    currentLabel = 'floating-button';
    let finishRequest!: () => void;
    getLatestPanelContextMock.mockResolvedValue({
      selection: {
        id: 'selection-replace',
        text: '你好世界',
        sourceApp: 'manual',
        windowTitle: 'Manual hotkey',
        fallbackPoint: { x: 320, y: 240 },
      },
      action: 'translateExplain',
    });
    runAiActionMock.mockImplementation(
      (request: { requestId: string }) =>
        new Promise((resolve) => {
          finishRequest = () => {
            emit('ai_stream_delta', { requestId: request.requestId, delta: 'Hello world' });
            emit('ai_stream_done', { requestId: request.requestId });
            resolve({ requestId: request.requestId });
          };
        }),
    );

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: '翻译并替换文本' }));

    expect(screen.getByRole('button', { name: '翻译并替换文本' })).toHaveTextContent('替换中…');
    await waitFor(() => expect(runAiActionMock).toHaveBeenCalledTimes(1));
    await act(async () => finishRequest());
    await waitFor(() => expect(replaceSelectedTextMock).toHaveBeenCalledWith('Hello world', 'selection-replace'));
  });

  it('saves replacement target from the compact replacement preset window without closing it', async () => {
    currentLabel = 'replacement-preset';
    saveAppBehaviorConfigMock.mockImplementation(async (preferences) => ({
      defaultProviderId: null,
      providers: [],
      hoverRadius: 90,
      hoverDelayMs: 220,
      candidateTimeoutMs: 4000,
      minDragDistance: 6,
      clipboardFallbackEnabled: true,
      showClipboardPrivacyWarningOnFirstUse: true,
      disableInElevatedWindows: true,
      manualHotkeyAlwaysEnabled: true,
      disabledApps: [],
      ...preferences,
    }));

    render(<App />);
    fireEvent.click(await screen.findByRole('button', { name: '日文' }));

    await waitFor(() =>
      expect(saveAppBehaviorConfigMock).toHaveBeenCalledWith(
        expect.objectContaining({ replacementTargetLanguage: 'japanese', replacementCustomTarget: '' }),
      ),
    );
    expect(hideReplacementPresetPanelMock).not.toHaveBeenCalled();
  });

  it('does not replace selected text when the replace stream reports an error', async () => {
    currentLabel = 'floating-button';
    getLatestPanelContextMock.mockResolvedValue({
      selection: {
        text: '你好世界',
        sourceApp: 'manual',
        windowTitle: 'Manual hotkey',
        fallbackPoint: { x: 320, y: 240 },
      },
      action: 'translateExplain',
    });
    runAiActionMock.mockImplementation(async (request: { requestId: string }) => {
      emit('ai_stream_delta', { requestId: request.requestId, delta: 'partial' });
      emit('ai_stream_error', { requestId: request.requestId, code: 'provider_stream_failed', message: 'failed' });
      emit('ai_stream_done', { requestId: request.requestId });
      return { requestId: request.requestId };
    });

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: '翻译并替换文本' }));

    await waitFor(() => expect(runAiActionMock).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(replaceSelectedTextMock).not.toHaveBeenCalled());
  });

  it('renders translate result window as a movable single-column translation popup', async () => {
    currentLabel = 'translate-result';

    render(<App />);
    await waitFor(() => expect(listeners.has('translate_result')).toBe(true));

    await act(async () => {
      emit('translate_result', { originalText: 'hello world', translatedText: '你好世界' });
    });

    const header = screen.getByTitle('拖拽移动翻译浮窗');
    fireEvent.mouseDown(header);
    fireEvent.mouseDown(screen.getByRole('button', { name: '调整翻译浮窗大小' }));
    fireEvent.click(screen.getByRole('button', { name: '关闭翻译浮窗' }));

    expect(screen.getByText('译文')).toBeInTheDocument();
    expect(screen.getByLabelText('译文内容')).toHaveTextContent('你好世界');
    expect(screen.queryByText('原文')).not.toBeInTheDocument();
    expect(screen.queryByText('hello world')).not.toBeInTheDocument();
    expect(currentWindowStartDraggingMock).toHaveBeenCalledTimes(1);
    expect(currentWindowStartResizeDraggingMock).toHaveBeenCalledWith('SouthEast');
    expect(hideTranslateResultMock).toHaveBeenCalledTimes(1);
  });

  it('renders AI panel for the ai-panel window', () => {
    currentLabel = 'ai-panel';

    render(<App />);

    expect(screen.getByText(/点击“执行当前动作”开始生成。/)).toBeInTheDocument();
  });

  it('shows running feedback after receiving selected text and clicking an AI panel action', async () => {
    currentLabel = 'ai-panel';

    render(<App />);
    await waitFor(() => expect(listeners.has('panel_context')).toBe(true));

    await act(async () => {
      emit('panel_context', {
        selection: { text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'translateExplain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '总结' }));
    expect(runAiActionMock).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

    await waitFor(() =>
      expect(runAiActionMock).toHaveBeenCalledWith(
        expect.objectContaining({ action: 'summarize', text: 'hello world', requestId: expect.any(String) }),
      ),
    );
    expect(screen.getByText(/生成中/)).toBeInTheDocument();
    expect(screen.getByText('动作：总结')).toBeInTheDocument();
  });

  it('renders source text window for the source-text window and displays pushed source text', async () => {
    currentLabel = 'source-text';

    render(<App />);
    await waitFor(() => expect(listeners.has('source_text_context')).toBe(true));

    await act(async () => {
      emit('source_text_context', { text: 'hello source text' });
    });

    expect(screen.getByRole('heading', { name: '原文' })).toBeInTheDocument();
    expect(screen.getByText('hello source text')).toBeInTheDocument();
  });

  it('updates an open source text window when a newer panel context arrives', async () => {
    currentLabel = 'source-text';

    render(<App />);
    await waitFor(() => expect(listeners.has('panel_context')).toBe(true));

    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-b', text: 'new selected source text', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'summarize',
        autoRun: false,
      });
    });

    expect(screen.getByText('new selected source text')).toBeInTheDocument();
  });

  it('recovers source text when the source window opens after the context event was missed', async () => {
    currentLabel = 'source-text';
    getLatestSourceTextContextMock.mockResolvedValue({ text: 'missed source text' });

    render(<App />);

    expect(await screen.findByText('missed source text')).toBeInTheDocument();
    expect(getLatestSourceTextContextMock).toHaveBeenCalledTimes(1);
  });

  it('recovers source text when a shown source window gains focus after missing the context event', async () => {
    currentLabel = 'source-text';
    getLatestSourceTextContextMock
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ text: 'focused recovered source text' });

    render(<App />);

    expect(await screen.findByText('等待原文内容')).toBeInTheDocument();
    await waitFor(() => expect(focusListeners).toHaveLength(1));

    await act(async () => {
      emitFocusChanged(true);
    });

    expect(await screen.findByText('focused recovered source text')).toBeInTheDocument();
    expect(getLatestSourceTextContextMock).toHaveBeenCalledTimes(2);
  });

  it('renders settings for other windows', async () => {
    render(<App />);

    expect(await screen.findByRole('heading', { name: '设置' })).toBeInTheDocument();
  });
});
