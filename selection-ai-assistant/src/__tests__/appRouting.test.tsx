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
  hideSourceTextWindowMock,
  getLatestSourceTextContextMock,
} = vi.hoisted(() => ({
  listeners: new Map<string, Listener[]>(),
  focusListeners: [] as Listener<boolean>[],
  openPanelFromFloatingButtonMock: vi.fn(),
  runAiActionMock: vi.fn(),
  hideSourceTextWindowMock: vi.fn(),
  getLatestSourceTextContextMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    label: currentLabel,
    startDragging: vi.fn(),
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
      disabledApps: [],
    }),
  ),
  saveProviderConfig: vi.fn(),
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
  runAiAction: runAiActionMock,
  runAiFollowUp: vi.fn(),
  getLatestPanelContext: vi.fn(() => Promise.resolve(null)),
  getLatestSourceTextContext: getLatestSourceTextContextMock,
  openPanelFromFloatingButton: openPanelFromFloatingButtonMock,
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
    openPanelFromFloatingButtonMock.mockReset();
    hideSourceTextWindowMock.mockReset();
    getLatestSourceTextContextMock.mockReset();
    runAiActionMock.mockResolvedValue({ requestId: 'request-1' });
    openPanelFromFloatingButtonMock.mockResolvedValue(undefined);
    hideSourceTextWindowMock.mockResolvedValue(undefined);
    getLatestSourceTextContextMock.mockResolvedValue(null);
  });

  it('renders floating button for the floating-button window', () => {
    currentLabel = 'floating-button';

    render(<App />);

    expect(screen.getByRole('button', { name: '打开 AI 助手' })).toBeInTheDocument();
  });

  it('renders floating button inside a transparent window root container', () => {
    currentLabel = 'floating-button';

    render(<App />);

    const button = screen.getByRole('button', { name: '打开 AI 助手' });
    expect(button).toHaveClass('floating-ai-button');
    expect(button.closest('.floating-button-window')).not.toBeNull();
  });

  it('renders floating button as an icon-only accessible button without image assets', () => {
    currentLabel = 'floating-button';

    render(<App />);

    const button = screen.getByRole('button', { name: '打开 AI 助手' });
    expect(button).toHaveClass('floating-ai-button');
    expect(button.closest('.floating-button-window')).not.toBeNull();
    expect(button.querySelector('img')).toBeNull();
    expect(button).not.toHaveTextContent('AI');

    const svg = button.querySelector('svg.floating-ai-mark-icon');
    expect(svg).not.toBeNull();
    expect(svg?.closest('[aria-hidden="true"]')).not.toBeNull();
  });

  it('opens the panel when the floating button is clicked', async () => {
    currentLabel = 'floating-button';

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: '打开 AI 助手' }));

    await waitFor(() => expect(openPanelFromFloatingButtonMock).toHaveBeenCalledTimes(1));
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
