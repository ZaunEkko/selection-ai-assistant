import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from '../App';

let currentLabel = 'main';
type Listener<T = unknown> = (event: { payload: T }) => void;

const { listeners, openPanelFromFloatingButtonMock, runAiActionMock } = vi.hoisted(() => ({
  listeners: new Map<string, Listener[]>(),
  openPanelFromFloatingButtonMock: vi.fn(),
  runAiActionMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ label: currentLabel }),
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
      disabledApps: [],
    }),
  ),
  saveProviderConfig: vi.fn(),
  runAiAction: runAiActionMock,
  openPanelFromFloatingButton: openPanelFromFloatingButtonMock,
  formatCommandError: (err: unknown) => (err instanceof Error ? err.message : String(err)),
}));

function emit<T>(eventName: string, payload: T) {
  for (const listener of listeners.get(eventName) ?? []) {
    listener({ payload });
  }
}

describe('App routing by Tauri window label', () => {
  beforeEach(() => {
    currentLabel = 'main';
    listeners.clear();
    runAiActionMock.mockReset();
    openPanelFromFloatingButtonMock.mockReset();
    runAiActionMock.mockResolvedValue({ requestId: 'request-1' });
    openPanelFromFloatingButtonMock.mockResolvedValue(undefined);
  });

  it('renders floating button for the floating-button window', () => {
    currentLabel = 'floating-button';

    render(<App />);

    expect(screen.getByRole('button', { name: /open ai assistant/i })).toBeInTheDocument();
  });

  it('opens the panel when the floating button is clicked', async () => {
    currentLabel = 'floating-button';

    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: /open ai assistant/i }));

    await waitFor(() => expect(openPanelFromFloatingButtonMock).toHaveBeenCalledTimes(1));
  });

  it('renders AI panel for the ai-panel window', () => {
    currentLabel = 'ai-panel';

    render(<App />);

    expect(screen.getByText(/点击动作开始生成/)).toBeInTheDocument();
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

    await waitFor(() =>
      expect(runAiActionMock).toHaveBeenCalledWith(
        expect.objectContaining({ action: 'summarize', text: 'hello world', requestId: expect.any(String) }),
      ),
    );
    expect(screen.getByText(/生成中/)).toBeInTheDocument();
    expect(screen.getByText('动作：summarize')).toBeInTheDocument();
  });

  it('renders settings for other windows', async () => {
    render(<App />);

    expect(await screen.findByRole('heading', { name: /settings/i })).toBeInTheDocument();
  });
});
