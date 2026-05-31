import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AiPanel } from '../windows/AiPanel';

type Listener<T = unknown> = (event: { payload: T }) => void;

const { invokeMock, listenMock, listeners } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  listeners: new Map<string, Listener[]>(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

function emit<T>(eventName: string, payload: T) {
  for (const listener of listeners.get(eventName) ?? []) {
    listener({ payload });
  }
}

describe('AiPanel Tauri event lifecycle', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
    listeners.clear();
    vi.stubGlobal('crypto', { ...globalThis.crypto, randomUUID: () => 'frontend-request-1' });
    listenMock.mockImplementation((eventName: string, callback: Listener) => {
      const existing = listeners.get(eventName) ?? [];
      existing.push(callback);
      listeners.set(eventName, existing);
      return Promise.resolve(() => {
        listeners.set(
          eventName,
          (listeners.get(eventName) ?? []).filter((item) => item !== callback),
        );
      });
    });
    invokeMock.mockResolvedValue({ requestId: 'request-1' });
  });

  it('starts the frontend request before invoke settles and renders matching stream events', async () => {
    let resolveInvoke!: (value: { requestId: string }) => void;
    invokeMock.mockReturnValue(
      new Promise((resolve) => {
        resolveInvoke = resolve;
      }),
    );

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'translateExplain',
      });
    });

    expect(screen.getByText('hello world')).toBeInTheDocument();
    expect(screen.getByText('动作：translateExplain')).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: '总结' }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('run_ai_action', {
        request: { requestId: 'frontend-request-1', action: 'summarize', text: 'hello world' },
      });
    });
    expect(screen.getByText(/生成中/)).toBeInTheDocument();

    await act(async () => {
      emit('ai_stream_delta', { requestId: 'stale-request', delta: 'stale' });
      emit('ai_stream_delta', { requestId: 'frontend-request-1', delta: 'hello' });
      emit('ai_stream_delta', { requestId: 'frontend-request-1', delta: ' world' });
    });

    expect(screen.getAllByText('hello world').length).toBeGreaterThan(0);
    expect(screen.queryByText(/stale/)).not.toBeInTheDocument();

    await act(async () => {
      emit('ai_stream_done', { requestId: 'frontend-request-1' });
    });
    expect(screen.queryByText(/生成中/)).not.toBeInTheDocument();

    await act(async () => {
      resolveInvoke({ requestId: 'frontend-request-1' });
    });
  });

  it('auto-runs the classified action only when the panel context requests it', async () => {
    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { text: 'manual text', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'translateExplain',
      });
    });

    expect(invokeMock).not.toHaveBeenCalledWith('run_ai_action', expect.anything());

    await act(async () => {
      emit('panel_context', {
        selection: { text: 'auto text', sourceApp: 'unknown', windowTitle: 'Unknown window' },
        action: 'summarize',
        autoRun: true,
      });
    });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('run_ai_action', {
        request: { requestId: 'frontend-request-1', action: 'summarize', text: 'auto text' },
      });
    });
  });

  it('does not let a stale invoke rejection reset a newer request', async () => {
    const requestIds = ['request-1', 'request-2'];
    vi.stubGlobal('crypto', { ...globalThis.crypto, randomUUID: () => requestIds.shift() ?? 'fallback-request' });

    let rejectFirst!: (reason?: unknown) => void;
    invokeMock
      .mockReturnValueOnce(
        new Promise((_, reject) => {
          rejectFirst = reject;
        }),
      )
      .mockReturnValueOnce(new Promise(() => undefined));

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'translateExplain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '总结' }));
    fireEvent.click(screen.getByRole('button', { name: '解释' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(2));

    await act(async () => {
      rejectFirst({ code: 'first_failed', message: 'first request failed' });
    });

    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByText(/生成中/)).toBeInTheDocument();
    expect(screen.getByText('动作：explain')).toBeInTheDocument();
  });

  it('hides the AI panel when the close button is clicked', async () => {
    render(<AiPanel />);

    fireEvent.click(screen.getByRole('button', { name: 'Close panel' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('hide_ai_panel'));
  });

  it('does not call run_ai_action without selected text', async () => {
    render(<AiPanel />);

    fireEvent.click(screen.getByRole('button', { name: '解释' }));

    await waitFor(() => expect(listenMock).toHaveBeenCalled());
    expect(invokeMock).not.toHaveBeenCalledWith('run_ai_action', expect.anything());
  });
});
