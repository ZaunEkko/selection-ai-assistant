import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AiPanel } from '../windows/AiPanel';

type Listener<T = unknown> = (event: { payload: T }) => void;

const { invokeMock, listenMock, startDraggingMock, listeners } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn(),
  startDraggingMock: vi.fn(),
  listeners: new Map<string, Listener[]>(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ startDragging: startDraggingMock }),
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
    startDraggingMock.mockReset();
    vi.useRealTimers();
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
    expect(screen.getByText('动作：翻译解释')).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: '总结' }));
    expect(screen.getByText('动作：总结')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '总结' })).toHaveAttribute('aria-pressed', 'true');
    expect(invokeMock).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

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
    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    fireEvent.click(screen.getByRole('button', { name: '解释' }));
    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(2));

    await act(async () => {
      rejectFirst({ code: 'first_failed', message: 'first request failed' });
    });

    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByText(/生成中/)).toBeInTheDocument();
    expect(screen.getByText('动作：解释')).toBeInTheDocument();
  });

  it('hides the AI panel when the close button is clicked', async () => {
    render(<AiPanel />);

    fireEvent.click(screen.getByRole('button', { name: '关闭面板' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('hide_ai_panel'));
  });

  it('selects an action without running it until the execute button is clicked', async () => {
    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '代码解释' }));

    expect(screen.getByText('动作：代码解释')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '代码解释' })).toHaveAttribute('aria-pressed', 'true');
    expect(invokeMock).not.toHaveBeenCalledWith('run_ai_action', expect.anything());

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('run_ai_action', {
        request: { requestId: 'frontend-request-1', action: 'codeExplain', text: 'hello world' },
      });
    });
  });

  it('selects expand prompt action and runs it only after execute click', async () => {
    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: {
          id: 'sel-expand-prompt',
          text: '帮我写一个产品发布的提示词',
          sourceApp: 'manual',
          windowTitle: 'Manual hotkey',
        },
        action: 'explain',
        autoRun: false,
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '扩写提示词' }));

    expect(screen.getByText('动作：扩写提示词')).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith('run_ai_action', expect.anything());

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('run_ai_action', {
        request: {
          requestId: 'frontend-request-1',
          action: 'expandPrompt',
          text: '帮我写一个产品发布的提示词',
        },
      });
    });
  });

  it('shows a prompt and does not call the backend when executing without selected text', async () => {
    render(<AiPanel />);

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('请先选中文本后再执行 AI 动作。');
    expect(invokeMock).not.toHaveBeenCalledWith('run_ai_action', expect.anything());
  });

  it('recovers the stored selection when the panel context event was missed before executing', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_latest_panel_context') {
        return Promise.resolve({
          selection: { text: 'missed selected text', sourceApp: 'chrome.exe', windowTitle: 'Browser' },
          action: 'summarize',
          autoRun: true,
        });
      }
      if (command === 'run_ai_action') return Promise.resolve({ requestId: 'frontend-request-1' });
      return Promise.resolve(undefined);
    });

    render(<AiPanel />);

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

    expect(await screen.findByText('missed selected text')).toBeInTheDocument();
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('run_ai_action', {
        request: { requestId: 'frontend-request-1', action: 'summarize', text: 'missed selected text' },
      });
    });
    expect(screen.queryByText('请先选中文本后再执行 AI 动作。')).not.toBeInTheDocument();
  });

  it('shows stream errors for the active request and stops loading', async () => {
    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('run_ai_action', expect.anything()));
    expect(screen.getByText(/生成中/)).toBeInTheDocument();

    await act(async () => {
      emit('ai_stream_error', {
        requestId: 'frontend-request-1',
        code: 'provider_stream_failed',
        message: '服务商请求失败，请检查配置后重试。',
      });
    });

    expect(await screen.findByRole('alert')).toHaveTextContent('AI 服务商请求失败：服务商请求失败，请检查配置后重试。');
    expect(screen.queryByText(/生成中/)).not.toBeInTheDocument();
  });

  it('does not let a stale stream error override the current request', async () => {
    const requestIds = ['request-1', 'request-2'];
    vi.stubGlobal('crypto', { ...globalThis.crypto, randomUUID: () => requestIds.shift() ?? 'fallback-request' });
    invokeMock.mockReturnValue(new Promise(() => undefined));

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'translateExplain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '总结' }));
    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    fireEvent.click(screen.getByRole('button', { name: '解释' }));
    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(2));

    await act(async () => {
      emit('ai_stream_error', {
        requestId: 'request-1',
        code: 'provider_stream_failed',
        message: 'first request failed',
      });
    });

    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByText(/生成中/)).toBeInTheDocument();
    expect(screen.getByText('动作：解释')).toBeInTheDocument();
  });

  it('starts dragging from the panel header but not from the close button', async () => {
    render(<AiPanel />);

    fireEvent.mouseDown(screen.getByTitle('拖拽移动面板'));
    await waitFor(() => expect(startDraggingMock).toHaveBeenCalledTimes(1));

    fireEvent.mouseDown(screen.getByRole('button', { name: '关闭面板' }));
    expect(startDraggingMock).toHaveBeenCalledTimes(1);
  });

  it('stops loading and shows an error when a stream request times out', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_latest_panel_context') return Promise.resolve(null);
      if (command === 'run_ai_action') return Promise.resolve({ requestId: 'frontend-request-1' });
      return Promise.resolve(undefined);
    });

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    vi.useFakeTimers();
    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await act(async () => undefined);
    expect(screen.getByText(/生成中/)).toBeInTheDocument();

    await act(async () => {
      vi.advanceTimersByTime(60_000);
    });

    expect(screen.getByRole('alert')).toHaveTextContent('AI 请求超时，请稍后重试或检查服务商配置。');
    expect(screen.queryByText(/生成中/)).not.toBeInTheDocument();
  });

  it('uses the latest stored selection before executing when the visible context is stale', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_latest_panel_context') {
        return Promise.resolve({
          selection: { id: 'selection-b', text: 'new selected text', sourceApp: 'chrome.exe', windowTitle: 'Browser' },
          action: 'summarize',
        });
      }
      if (command === 'run_ai_action') return Promise.resolve({ requestId: 'frontend-request-1' });
      return Promise.resolve(undefined);
    });

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'old selected text', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

    expect(await screen.findByText('new selected text')).toBeInTheDocument();
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('run_ai_action', {
        request: { requestId: 'frontend-request-1', action: 'summarize', text: 'new selected text' },
      });
    });
  });

  it('syncs a different visible selection during generation without rerunning or replacing the active stream', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_latest_panel_context') return Promise.resolve(null);
      if (command === 'run_ai_action') return Promise.resolve({ requestId: 'frontend-request-1' });
      return Promise.resolve(undefined);
    });

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'old selected text', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await waitFor(() => expect(screen.getByText(/生成中/)).toBeInTheDocument());

    await act(async () => {
      emit('ai_stream_delta', { requestId: 'frontend-request-1', delta: 'old answer' });
      emit('panel_context', {
        selection: { id: 'selection-b', text: 'new selected text', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'summarize',
        autoRun: false,
      });
      emit('ai_stream_delta', { requestId: 'frontend-request-1', delta: ' continues' });
      emit('ai_stream_done', { requestId: 'frontend-request-1' });
    });

    const runCalls = invokeMock.mock.calls.filter(([command]) => command === 'run_ai_action');
    expect(runCalls).toHaveLength(1);
    expect(screen.getByText('new selected text')).toBeInTheDocument();
    expect(screen.getByText(/old answer continues/)).toBeInTheDocument();
    expect(screen.getByText('动作：总结')).toBeInTheDocument();
    expect(screen.queryByText(/生成中/)).not.toBeInTheDocument();
  });

  it('shows a formatted error and stops loading when run_ai_action rejects', async () => {
    invokeMock.mockRejectedValueOnce({ code: 'api_key_missing', message: 'missing key' });

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('请在设置中填写 API 密钥，或配置 SELECTION_AI_API_KEY 环境变量。');
    expect(screen.queryByText(/生成中/)).not.toBeInTheDocument();
  });

  it('sends a follow-up question with the selected text and previous answer', async () => {
    const requestIds = ['initial-request', 'follow-up-request'];
    vi.stubGlobal('crypto', { ...globalThis.crypto, randomUUID: () => requestIds.shift() ?? 'fallback-request' });
    invokeMock.mockResolvedValue({ requestId: 'ok' });

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('run_ai_action', {
        request: { requestId: 'initial-request', action: 'explain', text: 'hello world' },
      });
    });

    await act(async () => {
      emit('ai_stream_delta', { requestId: 'initial-request', delta: '初始回答' });
      emit('ai_stream_done', { requestId: 'initial-request' });
    });

    fireEvent.change(screen.getByPlaceholderText('追问'), { target: { value: '继续解释' } });
    fireEvent.click(screen.getByRole('button', { name: '发送' }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('run_ai_follow_up', {
        request: {
          requestId: 'follow-up-request',
          originalText: 'hello world',
          previousAnswer: '初始回答',
          question: '继续解释',
        },
      });
    });
    expect(screen.getByPlaceholderText('追问')).toHaveValue('');
    expect(screen.getByText(/追问：继续解释/)).toBeInTheDocument();

    await act(async () => {
      emit('ai_stream_delta', { requestId: 'follow-up-request', delta: '补充回答' });
      emit('ai_stream_done', { requestId: 'follow-up-request' });
    });

    expect(screen.getByText(/初始回答/)).toBeInTheDocument();
    expect(screen.getByText(/补充回答/)).toBeInTheDocument();
  });

  it('sends a follow-up question when Enter is pressed', async () => {
    const requestIds = ['initial-request', 'follow-up-request'];
    vi.stubGlobal('crypto', { ...globalThis.crypto, randomUUID: () => requestIds.shift() ?? 'fallback-request' });

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('run_ai_action', expect.anything()));

    await act(async () => {
      emit('ai_stream_delta', { requestId: 'initial-request', delta: '初始回答' });
      emit('ai_stream_done', { requestId: 'initial-request' });
    });

    fireEvent.change(screen.getByPlaceholderText('追问'), { target: { value: '继续解释' } });
    fireEvent.keyDown(screen.getByPlaceholderText('追问'), { key: 'Enter', code: 'Enter' });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('run_ai_follow_up', {
        request: {
          requestId: 'follow-up-request',
          originalText: 'hello world',
          previousAnswer: '初始回答',
          question: '继续解释',
        },
      });
    });
  });

  it('auto-scrolls streamed answer output while the reader stays near the bottom', async () => {
    const { container } = render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('run_ai_action', expect.anything()));
    const answer = container.querySelector('.ai-answer') as HTMLElement;
    Object.defineProperty(answer, 'scrollHeight', { configurable: true, value: 800 });
    Object.defineProperty(answer, 'clientHeight', { configurable: true, value: 200 });

    await act(async () => {
      emit('ai_stream_delta', { requestId: 'frontend-request-1', delta: '第一段' });
    });

    await waitFor(() => expect(answer.scrollTop).toBe(800));
  });

  it('does not force auto-scroll after the reader scrolls away from the bottom', async () => {
    const { container } = render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('run_ai_action', expect.anything()));
    const answer = container.querySelector('.ai-answer') as HTMLElement;
    Object.defineProperty(answer, 'scrollHeight', { configurable: true, value: 800 });
    Object.defineProperty(answer, 'clientHeight', { configurable: true, value: 200 });

    answer.scrollTop = 0;
    fireEvent.scroll(answer);

    await act(async () => {
      emit('ai_stream_delta', { requestId: 'frontend-request-1', delta: '读者正在看上文' });
    });

    expect(answer.scrollTop).toBe(0);
  });

  it('renders streamed markdown strong text without exposing marker syntax', async () => {
    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('run_ai_action', expect.anything()));
    await act(async () => {
      emit('ai_stream_delta', {
        requestId: 'frontend-request-1',
        delta: '这是 **重点内容** 和普通文字',
      });
    });

    expect(screen.getByText('重点内容').tagName).toBe('STRONG');
    expect(screen.queryByText(/\*\*重点内容\*\*/)).not.toBeInTheDocument();
  });

  it('renders streamed markdown as preview content including safe images', async () => {
    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('run_ai_action', expect.anything()));
    await act(async () => {
      emit('ai_stream_delta', {
        requestId: 'frontend-request-1',
        delta: '# 标题\n\n![预览图](https://example.com/preview.png)\n\n- 要点一',
      });
    });

    expect(screen.getByRole('heading', { name: '标题' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: '预览图' })).toHaveAttribute('src', 'https://example.com/preview.png');
    expect(screen.getByText('要点一')).toBeInTheDocument();
  });

  it('renders streamed markdown tables with visible table semantics', async () => {
    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('run_ai_action', expect.anything()));
    await act(async () => {
      emit('ai_stream_delta', {
        requestId: 'frontend-request-1',
        delta: '| 名称 | 说明 |\n| --- | --- |\n| Alpha | 第一项 |',
      });
    });

    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: '名称' })).toBeInTheDocument();
    expect(screen.getByRole('cell', { name: '第一项' })).toBeInTheDocument();
  });

  it('does not render unsafe markdown image urls', async () => {
    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('run_ai_action', expect.anything()));
    await act(async () => {
      emit('ai_stream_delta', {
        requestId: 'frontend-request-1',
        delta: '![危险图](javascript:alert(1))',
      });
    });

    expect(screen.queryByRole('img', { name: '危险图' })).not.toBeInTheDocument();
    expect(screen.getByText(/已隐藏不安全图片/)).toBeInTheDocument();
  });

  it('collapses long selected text by default and expands it on click', async () => {
    const longText = '这是一段很长的选中文本。'.repeat(12);

    const { container } = render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: longText, sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    const preview = container.querySelector('.selected-text-preview');
    expect(preview).toHaveClass('is-collapsed');
    expect(screen.getByRole('button', { name: '显示完整原文' })).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByText(longText)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '显示完整原文' }));

    expect(preview).toHaveClass('is-expanded');
    expect(screen.getByRole('button', { name: '收起原文' })).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText(longText)).toBeInTheDocument();
  });

  it('opens selected text in a separate left source window without sending it to the provider', async () => {
    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    const openSourceButton = screen.getByRole('button', { name: '在左侧窗口打开原文' });
    expect(openSourceButton.closest('.selected-text-card')).not.toBeNull();
    expect(openSourceButton.closest('.panel-controls')).toBeNull();

    fireEvent.click(openSourceButton);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('show_source_text_window', { text: 'hello world' });
    });
    expect(screen.queryByRole('region', { name: '左侧原文' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '关闭左侧原文' }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('hide_source_text_window');
    });
    expect(screen.getByRole('button', { name: '在左侧窗口打开原文' })).toHaveAttribute('aria-pressed', 'false');
    expect(invokeMock).not.toHaveBeenCalledWith('run_ai_action', expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith('run_ai_follow_up', expect.anything());
  });

  it('opens the latest stored selection in the source window when the visible panel text is stale', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_latest_panel_context') {
        return Promise.resolve({
          selection: { id: 'selection-b', text: 'second selected text', sourceApp: 'chrome.exe', windowTitle: 'Browser' },
          action: 'summarize',
        });
      }
      return Promise.resolve(undefined);
    });

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'first selected text', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '在左侧窗口打开原文' }));

    expect(await screen.findByText('second selected text')).toBeInTheDocument();
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('show_source_text_window', { text: 'second selected text' });
    });
    expect(invokeMock).not.toHaveBeenCalledWith('run_ai_action', expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith('run_ai_follow_up', expect.anything());
  });

  it('ignores same-selection panel refreshes while an answer is already generating', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_latest_panel_context') return Promise.resolve(null);
      if (command === 'run_ai_action') return Promise.resolve({ requestId: 'frontend-request-1' });
      return Promise.resolve(undefined);
    });

    render(<AiPanel />);

    await waitFor(() => expect(listenMock).toHaveBeenCalledWith('panel_context', expect.any(Function)));
    await act(async () => {
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'explain',
      });
    });

    fireEvent.click(screen.getByRole('button', { name: '执行当前动作' }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('run_ai_action', expect.anything()));

    await act(async () => {
      emit('ai_stream_delta', { requestId: 'frontend-request-1', delta: '第一段' });
      emit('panel_context', {
        selection: { id: 'selection-a', text: 'hello world', sourceApp: 'manual', windowTitle: 'Manual hotkey' },
        action: 'summarize',
        autoRun: false,
      });
      emit('ai_stream_delta', { requestId: 'frontend-request-1', delta: '第二段' });
    });

    const runCalls = invokeMock.mock.calls.filter(([command]) => command === 'run_ai_action');
    expect(runCalls).toHaveLength(1);
    expect(screen.getByText(/第一段第二段/)).toBeInTheDocument();
    expect(screen.getByText('动作：解释')).toBeInTheDocument();
  });
});
