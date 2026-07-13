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
  setReplacementPresetPanelExpandedMock,
  focusFloatingButtonMock,
  hideReplacementPresetPanelMock,
  showTranslateResultMock,
  hideTranslateResultMock,
  hideFloatingButtonMock,
  replaceSelectedTextMock,
  hideSourceTextWindowMock,
  cancelScreenshotTranslateMock,
  runScreenshotTranslateMock,
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
  setReplacementPresetPanelExpandedMock: vi.fn(),
  focusFloatingButtonMock: vi.fn(),
  hideReplacementPresetPanelMock: vi.fn(),
  showTranslateResultMock: vi.fn(),
  hideTranslateResultMock: vi.fn(),
  hideFloatingButtonMock: vi.fn(),
  replaceSelectedTextMock: vi.fn(),
  hideSourceTextWindowMock: vi.fn(),
  cancelScreenshotTranslateMock: vi.fn(),
  runScreenshotTranslateMock: vi.fn(),
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
      launchAtStartup: false,
      clipboardFallbackEnabled: true,
      showClipboardPrivacyWarningOnFirstUse: true,
      disableInElevatedWindows: true,
      manualHotkeyAlwaysEnabled: true,
      startMinimizedToTray: false,
      closeButtonBehavior: 'ask',
      replacementTargetLanguage: 'korean',
      replacementCustomTarget: '',
      translationTargetLanguage: 'morseCode',
      translationCustomTarget: '',
      disabledApps: [],
    }),
  ),
  getRuntimePreferences: vi.fn(() =>
    Promise.resolve({
      hotkey: 'Ctrl+Alt+A',
      launchAtStartup: false,
      startMinimizedToTray: false,
      closeButtonBehavior: 'ask',
      replacementTargetLanguage: 'korean',
      replacementCustomTarget: '',
      translationTargetLanguage: 'morseCode',
      translationCustomTarget: '',
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
  cancelScreenshotTranslate: cancelScreenshotTranslateMock,
  runScreenshotTranslate: runScreenshotTranslateMock,
  showScreenshotOverlay: vi.fn(),
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
  setReplacementPresetPanelExpanded: setReplacementPresetPanelExpandedMock,
  focusFloatingButton: focusFloatingButtonMock,
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
    setReplacementPresetPanelExpandedMock.mockReset();
    focusFloatingButtonMock.mockReset();
    hideReplacementPresetPanelMock.mockReset();
    hideFloatingButtonMock.mockReset();
    replaceSelectedTextMock.mockReset();
    openPanelFromFloatingButtonMock.mockReset();
    cancelScreenshotTranslateMock.mockReset();
    runScreenshotTranslateMock.mockReset();
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
      launchAtStartup: false,
      clipboardFallbackEnabled: true,
      showClipboardPrivacyWarningOnFirstUse: true,
      disableInElevatedWindows: true,
      manualHotkeyAlwaysEnabled: true,
      startMinimizedToTray: false,
      closeButtonBehavior: 'ask',
      replacementTargetLanguage: 'korean',
      replacementCustomTarget: '',
      translationTargetLanguage: 'auto',
      translationCustomTarget: '',
      disabledApps: [],
    });
    showTranslateResultMock.mockResolvedValue(undefined);
    showReplacementPresetPanelMock.mockResolvedValue(undefined);
    setReplacementPresetPanelExpandedMock.mockResolvedValue(undefined);
    focusFloatingButtonMock.mockResolvedValue(undefined);
    hideReplacementPresetPanelMock.mockResolvedValue(undefined);
    hideTranslateResultMock.mockResolvedValue(undefined);
    hideFloatingButtonMock.mockResolvedValue(undefined);
    replaceSelectedTextMock.mockResolvedValue(undefined);
    openPanelFromFloatingButtonMock.mockResolvedValue(undefined);
    cancelScreenshotTranslateMock.mockResolvedValue(undefined);
    runScreenshotTranslateMock.mockResolvedValue({ requestId: 'screenshot-request' });
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

  it('opens and switches the target preset panel context from compact buttons', async () => {
    vi.useFakeTimers();
    currentLabel = 'floating-button';

    try {
      render(<App />);
      const replaceButton = screen.getByRole('button', { name: '翻译并替换文本' });
      fireEvent.mouseEnter(replaceButton);

      expect(showReplacementPresetPanelMock).not.toHaveBeenCalled();
      await act(async () => {
        vi.advanceTimersByTime(150);
      });
      expect(showReplacementPresetPanelMock).toHaveBeenCalledWith('replacement');

      showReplacementPresetPanelMock.mockClear();
      const translateButton = screen.getByRole('button', { name: '翻译文本' });
      fireEvent.mouseMove(translateButton);

      expect(showReplacementPresetPanelMock).toHaveBeenCalledWith('translation');
    } finally {
      vi.useRealTimers();
    }
  });

  it('resets target preset tracking when the backend hides the secondary window', async () => {
    vi.useFakeTimers();
    currentLabel = 'floating-button';

    try {
      render(<App />);
      const replaceButton = screen.getByRole('button', { name: '翻译并替换文本' });
      fireEvent.mouseEnter(replaceButton);
      await act(async () => {
        vi.advanceTimersByTime(150);
      });
      expect(showReplacementPresetPanelMock).toHaveBeenCalledWith('replacement');

      await act(async () => {
        emit('target_preset_panel_hidden', null);
      });
      showReplacementPresetPanelMock.mockClear();
      fireEvent.mouseMove(replaceButton);
      await act(async () => {
        vi.advanceTimersByTime(150);
      });

      expect(showReplacementPresetPanelMock).toHaveBeenCalledWith('replacement');
    } finally {
      vi.useRealTimers();
    }
  });

  it('still executes translation after the target preset panel has opened', async () => {
    vi.useFakeTimers();
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

    try {
      render(<App />);
      const toolbar = screen.getByRole('toolbar', { name: '文本操作' });
      const translateButton = screen.getByRole('button', { name: '翻译文本' });
      fireEvent.mouseEnter(translateButton);
      await act(async () => {
        vi.advanceTimersByTime(150);
      });
      expect(showReplacementPresetPanelMock).toHaveBeenCalledWith('translation');

      vi.useRealTimers();
      fireEvent.click(translateButton);

      await waitFor(() => expect(getLatestPanelContextMock).toHaveBeenCalled());
      await waitFor(() =>
        expect(showTranslateResultMock).toHaveBeenCalledWith(
          { x: 320, y: 240 },
          'hello world',
          '',
          [],
        ),
      );
      expect(hideReplacementPresetPanelMock).toHaveBeenCalled();
      await waitFor(() => expect(showTranslateResultMock).toHaveBeenCalledTimes(2));

      showReplacementPresetPanelMock.mockClear();
      vi.useFakeTimers();
      fireEvent.mouseMove(translateButton);
      await act(async () => {
        vi.advanceTimersByTime(150);
      });
      expect(showReplacementPresetPanelMock).not.toHaveBeenCalled();

      fireEvent.mouseLeave(toolbar);
      fireEvent.mouseEnter(translateButton);
      await act(async () => {
        vi.advanceTimersByTime(150);
      });
      expect(showReplacementPresetPanelMock).toHaveBeenCalledWith('translation');
    } finally {
      vi.useRealTimers();
    }
  });

  it('switches the target preset panel when pointer movement is delivered to the toolbar container', async () => {
    vi.useFakeTimers();
    currentLabel = 'floating-button';
    let replaceRectSpy: ReturnType<typeof vi.spyOn> | undefined;
    let translateRectSpy: ReturnType<typeof vi.spyOn> | undefined;
    let moreRectSpy: ReturnType<typeof vi.spyOn> | undefined;

    try {
      render(<App />);
      const toolbar = screen.getByRole('toolbar', { name: '文本操作' });
      const replaceButton = screen.getByRole('button', { name: '翻译并替换文本' });
      const translateButton = screen.getByRole('button', { name: '翻译文本' });
      const moreButton = screen.getByRole('button', { name: '更多操作' });
      replaceRectSpy = vi.spyOn(replaceButton, 'getBoundingClientRect').mockReturnValue({
        left: 4,
        right: 70,
        top: 4,
        bottom: 42,
        width: 66,
        height: 38,
        x: 4,
        y: 4,
        toJSON: () => ({}),
      } as DOMRect);
      translateRectSpy = vi.spyOn(translateButton, 'getBoundingClientRect').mockReturnValue({
        left: 70,
        right: 136,
        top: 4,
        bottom: 42,
        width: 66,
        height: 38,
        x: 70,
        y: 4,
        toJSON: () => ({}),
      } as DOMRect);
      moreRectSpy = vi.spyOn(moreButton, 'getBoundingClientRect').mockReturnValue({
        left: 136,
        right: 190,
        top: 4,
        bottom: 42,
        width: 54,
        height: 38,
        x: 136,
        y: 4,
        toJSON: () => ({}),
      } as DOMRect);

      fireEvent.mouseMove(toolbar, { clientX: 30, clientY: 18 });
      await act(async () => {
        vi.advanceTimersByTime(150);
      });
      expect(showReplacementPresetPanelMock).toHaveBeenCalledWith('replacement');
      expect(replaceButton).toHaveClass('is-pointer-active');

      showReplacementPresetPanelMock.mockClear();
      await act(async () => {
        emit('floating_button_pointer_position', { x: 90, y: 18, width: window.innerWidth, height: window.innerHeight });
      });

      expect(showReplacementPresetPanelMock).toHaveBeenCalledWith('translation');
      expect(translateButton).toHaveClass('is-pointer-active');

      hideReplacementPresetPanelMock.mockClear();
      await act(async () => {
        emit('floating_button_pointer_position', { x: 150, y: 18, width: window.innerWidth, height: window.innerHeight });
      });

      expect(hideReplacementPresetPanelMock).toHaveBeenCalled();
      expect(moreButton).toHaveClass('is-pointer-active');
    } finally {
      replaceRectSpy?.mockRestore();
      translateRectSpy?.mockRestore();
      moreRectSpy?.mockRestore();
      vi.useRealTimers();
    }
  });

  it('closes the target preset panel before opening more actions', async () => {
    vi.useFakeTimers();
    currentLabel = 'floating-button';

    try {
      render(<App />);
      fireEvent.mouseEnter(screen.getByRole('button', { name: '翻译并替换文本' }));
      await act(async () => {
        vi.advanceTimersByTime(150);
      });
      expect(showReplacementPresetPanelMock).toHaveBeenCalledWith('replacement');

      const moreButton = screen.getByRole('button', { name: '更多操作' });
      fireEvent.mouseMove(moreButton);
      fireEvent.click(moreButton);
      await act(async () => {});

      expect(hideReplacementPresetPanelMock).toHaveBeenCalled();
      expect(openPanelFromFloatingButtonMock).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
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
      expect.objectContaining({
        action: 'translateOnly',
        text: 'hello world',
        targetLanguage: '摩斯密码',
        requestId: expect.stringMatching(/^translate-/),
      }),
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
      launchAtStartup: false,
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
    await waitFor(() => expect(focusFloatingButtonMock).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(setReplacementPresetPanelExpandedMock).toHaveBeenLastCalledWith(false));
    expect(hideReplacementPresetPanelMock).not.toHaveBeenCalled();
  });

  it('saves translation target from the shared target preset window without mutating replacement target', async () => {
    currentLabel = 'replacement-preset';
    saveAppBehaviorConfigMock.mockImplementation(async (preferences) => ({
      defaultProviderId: null,
      providers: [],
      hoverRadius: 90,
      hoverDelayMs: 220,
      candidateTimeoutMs: 4000,
      minDragDistance: 6,
      launchAtStartup: false,
      clipboardFallbackEnabled: true,
      showClipboardPrivacyWarningOnFirstUse: true,
      disableInElevatedWindows: true,
      manualHotkeyAlwaysEnabled: true,
      disabledApps: [],
      ...preferences,
    }));

    render(<App />);
    await waitFor(() => expect(listeners.has('target_preset_context')).toBe(true));
    await act(async () => {
      emit('target_preset_context', { kind: 'translation' });
    });
    fireEvent.click(await screen.findByRole('button', { name: '甲骨' }));

    await waitFor(() =>
      expect(saveAppBehaviorConfigMock).toHaveBeenCalledWith(
        expect.objectContaining({
          replacementTargetLanguage: 'korean',
          replacementCustomTarget: '',
          translationTargetLanguage: 'oracleBone',
          translationCustomTarget: '',
        }),
      ),
    );
    await waitFor(() => expect(focusFloatingButtonMock).toHaveBeenCalledTimes(1));
    expect(hideReplacementPresetPanelMock).not.toHaveBeenCalled();
  });

  it('saves custom translation target from the shared target preset window', async () => {
    currentLabel = 'replacement-preset';
    saveAppBehaviorConfigMock.mockImplementation(async (preferences) => ({
      defaultProviderId: null,
      providers: [],
      hoverRadius: 90,
      hoverDelayMs: 220,
      candidateTimeoutMs: 4000,
      minDragDistance: 6,
      launchAtStartup: false,
      clipboardFallbackEnabled: true,
      showClipboardPrivacyWarningOnFirstUse: true,
      disableInElevatedWindows: true,
      manualHotkeyAlwaysEnabled: true,
      disabledApps: [],
      ...preferences,
    }));

    render(<App />);
    await waitFor(() => expect(listeners.has('target_preset_context')).toBe(true));
    await act(async () => {
      emit('target_preset_context', { kind: 'translation' });
    });
    fireEvent.click(await screen.findByRole('button', { name: '自定' }));
    await waitFor(() => expect(setReplacementPresetPanelExpandedMock).toHaveBeenCalledWith(true));
    fireEvent.change(screen.getByRole('textbox'), { target: { value: '象形文字风格' } });
    fireEvent.click(screen.getByRole('button', { name: '保存' }));

    await waitFor(() =>
      expect(saveAppBehaviorConfigMock).toHaveBeenCalledWith(
        expect.objectContaining({
          replacementTargetLanguage: 'korean',
          replacementCustomTarget: '',
          translationTargetLanguage: 'custom',
          translationCustomTarget: '象形文字风格',
        }),
      ),
    );
    await waitFor(() => expect(focusFloatingButtonMock).toHaveBeenCalledTimes(1));
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

  it('renders screenshot overlay and waits for confirmation before translating a dragged region', async () => {
    currentLabel = 'screenshot-overlay';

    render(<App />);
    const overlay = screen.getByRole('application', { name: '截图翻译取景层' });

    fireEvent.mouseDown(overlay, { button: 0, clientX: 120, clientY: 20 });
    fireEvent.mouseMove(overlay, { clientX: 520, clientY: 100 });
    fireEvent.mouseUp(overlay);

    expect(runScreenshotTranslateMock).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: '取消本次截图翻译' })).toBeInTheDocument();
    expect(screen.getByLabelText('截图翻译目标').closest('.screenshot-confirm-controls')).toHaveStyle({
      left: '192px',
      top: '110px',
    });
    fireEvent.change(screen.getByLabelText('截图翻译目标'), { target: { value: 'morseCode' } });
    fireEvent.click(screen.getByRole('button', { name: '确认本次截图翻译' }));

    await waitFor(() =>
      expect(runScreenshotTranslateMock).toHaveBeenCalledWith({
        requestId: expect.stringMatching(/^screenshot-/),
        rect: { x: 120, y: 20, width: 400, height: 80 },
        viewportSize: { width: window.innerWidth, height: window.innerHeight },
        targetLanguage: '摩斯密码',
      }),
    );
    expect(cancelScreenshotTranslateMock).not.toHaveBeenCalled();
    expect(screen.getByText('拖拽框选不可选中的文字区域')).toBeInTheDocument();
  });

  it('keeps the screenshot overlay visible when translation validation fails', async () => {
    currentLabel = 'screenshot-overlay';
    runScreenshotTranslateMock.mockRejectedValueOnce(new Error('未配置可用的截图翻译服务商'));

    render(<App />);
    const overlay = screen.getByRole('application', { name: '截图翻译取景层' });

    fireEvent.mouseDown(overlay, { button: 0, clientX: 120, clientY: 20 });
    fireEvent.mouseMove(overlay, { clientX: 520, clientY: 100 });
    fireEvent.mouseUp(overlay);
    fireEvent.click(screen.getByRole('button', { name: '确认本次截图翻译' }));

    expect(await screen.findByText('未配置可用的截图翻译服务商')).toBeInTheDocument();
    expect(screen.getByRole('application', { name: '截图翻译取景层' })).toBeInTheDocument();
    expect(cancelScreenshotTranslateMock).not.toHaveBeenCalled();
  });

  it('resets pending screenshot selection so a second capture can drag a new region', async () => {
    currentLabel = 'screenshot-overlay';

    render(<App />);
    const overlay = screen.getByRole('application', { name: '截图翻译取景层' });

    fireEvent.mouseDown(overlay, { button: 0, clientX: 10, clientY: 20 });
    fireEvent.mouseMove(overlay, { clientX: 80, clientY: 100 });
    fireEvent.mouseUp(overlay);
    expect(runScreenshotTranslateMock).not.toHaveBeenCalled();

    fireEvent.mouseDown(overlay, { button: 0, clientX: 30, clientY: 40 });
    fireEvent.mouseMove(overlay, { clientX: 110, clientY: 160 });
    fireEvent.mouseUp(overlay);
    fireEvent.click(screen.getByRole('button', { name: '确认本次截图翻译' }));

    await waitFor(() => expect(runScreenshotTranslateMock).toHaveBeenCalledTimes(1));
    expect(runScreenshotTranslateMock).toHaveBeenLastCalledWith({
      requestId: expect.stringMatching(/^screenshot-/),
      rect: { x: 30, y: 40, width: 80, height: 120 },
      viewportSize: { width: window.innerWidth, height: window.innerHeight },
    });
  });

  it('cancels pending screenshot selection from the confirmation controls', async () => {
    currentLabel = 'screenshot-overlay';

    render(<App />);
    const overlay = screen.getByRole('application', { name: '截图翻译取景层' });

    fireEvent.mouseDown(overlay, { button: 0, clientX: 10, clientY: 20 });
    fireEvent.mouseMove(overlay, { clientX: 80, clientY: 100 });
    fireEvent.mouseUp(overlay);
    fireEvent.pointerDown(screen.getByRole('button', { name: '取消本次截图翻译' }));
    fireEvent.mouseDown(screen.getByRole('button', { name: '取消本次截图翻译' }), { button: 0, clientX: 70, clientY: 110 });
    fireEvent.click(screen.getByRole('button', { name: '取消本次截图翻译' }));

    await waitFor(() => expect(cancelScreenshotTranslateMock).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole('button', { name: '取消本次截图翻译' })).not.toBeInTheDocument();
    expect(runScreenshotTranslateMock).not.toHaveBeenCalled();
  });

  it('cancels screenshot overlay when Escape is pressed', async () => {
    currentLabel = 'screenshot-overlay';

    render(<App />);
    fireEvent.keyDown(window, { key: 'Escape' });

    await waitFor(() => expect(cancelScreenshotTranslateMock).toHaveBeenCalledTimes(1));
  });

  it('renders settings for other windows', async () => {
    render(<App />);

    expect(await screen.findByRole('heading', { name: '设置' })).toBeInTheDocument();
  });
});
