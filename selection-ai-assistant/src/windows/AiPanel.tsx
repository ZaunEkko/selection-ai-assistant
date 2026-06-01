import { listen } from '@tauri-apps/api/event';
import { useEffect, useReducer, useRef, useState, type MouseEvent } from 'react';
import {
  formatCommandError,
  getLatestPanelContext,
  hideAiPanel,
  runAiAction,
  startDragAiPanel,
  type PanelContext,
  type UiAction,
} from '../api/tauri';
import { ActionBar } from '../components/ActionBar';
import { actionLabels } from '../components/actionLabels';
import { initialPanelState, panelReducer } from '../stores/panelStore';

type StreamDelta = { requestId: string; delta: string };
type StreamError = { requestId: string; code: string; message: string };
type StreamDone = { requestId: string };

const AI_STREAM_TIMEOUT_MS = 60_000;
const INTERACTIVE_HEADER_SELECTOR = 'button, input, textarea, select, a, [role="button"]';

export function AiPanel() {
  const [panel, dispatchPanel] = useReducer(panelReducer, initialPanelState);
  const [activeAction, setActiveAction] = useState<UiAction>('explain');
  const [selectedText, setSelectedText] = useState('');
  const [selectedSelectionId, setSelectedSelectionId] = useState<string | null>(null);
  const [question, setQuestion] = useState('');
  const [error, setError] = useState<string | null>(null);
  const activeRequestId = useRef<string | null>(null);
  const streamTimeoutId = useRef<number | null>(null);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;

    listen<PanelContext>('panel_context', (event) => {
      const { action, text } = applyPanelContext(event.payload);
      if (event.payload.autoRun === true) {
        void runActionForText(action, text);
      }
    }).then((fn) => {
      if (disposed) fn();
      else unlisteners.push(fn);
    });

    listen<StreamDelta>('ai_stream_delta', (event) => {
      dispatchPanel({ type: 'delta', requestId: event.payload.requestId, delta: event.payload.delta });
    }).then((fn) => {
      if (disposed) fn();
      else unlisteners.push(fn);
    });

    listen<StreamError>('ai_stream_error', (event) => {
      if (activeRequestId.current !== event.payload.requestId) return;
      activeRequestId.current = null;
      clearStreamTimeout();
      setError(formatCommandError(event.payload));
      dispatchPanel({ type: 'finish', requestId: event.payload.requestId });
    }).then((fn) => {
      if (disposed) fn();
      else unlisteners.push(fn);
    });

    listen<StreamDone>('ai_stream_done', (event) => {
      if (activeRequestId.current === event.payload.requestId) {
        activeRequestId.current = null;
        clearStreamTimeout();
      }
      dispatchPanel({ type: 'finish', requestId: event.payload.requestId });
    }).then((fn) => {
      if (disposed) fn();
      else unlisteners.push(fn);
    });

    return () => {
      disposed = true;
      clearStreamTimeout();
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  function clearStreamTimeout() {
    if (streamTimeoutId.current === null) return;
    window.clearTimeout(streamTimeoutId.current);
    streamTimeoutId.current = null;
  }

  function startStreamTimeout(requestId: string) {
    clearStreamTimeout();
    streamTimeoutId.current = window.setTimeout(() => {
      if (activeRequestId.current !== requestId) return;
      activeRequestId.current = null;
      streamTimeoutId.current = null;
      setError('AI 请求超时，请稍后重试或检查服务商配置。');
      dispatchPanel({ type: 'finish', requestId });
    }, AI_STREAM_TIMEOUT_MS);
  }

  function applyPanelContext(context: PanelContext, actionOverride?: UiAction) {
    const text = context.selection.text;
    const action = actionOverride ?? context.action;
    setSelectedText(text);
    setSelectedSelectionId(context.selection.id ?? null);
    setActiveAction(action);
    setError(null);
    activeRequestId.current = null;
    clearStreamTimeout();
    dispatchPanel({ type: 'reset' });
    return { action, text, selectionId: context.selection.id ?? null };
  }

  async function loadLatestPanelContext(actionOverride?: UiAction) {
    try {
      const context = await getLatestPanelContext();
      if (!context?.selection?.text?.trim()) return null;
      return applyPanelContext(context, actionOverride);
    } catch (err) {
      setError(formatCommandError(err));
      return null;
    }
  }

  async function runActionForText(action: UiAction, text: string, options: { showMissingTextError?: boolean } = {}) {
    if (!text.trim()) {
      if (options.showMissingTextError === true) {
        setError('请先选中文本后再执行 AI 动作。');
      }
      return;
    }

    setActiveAction(action);
    setError(null);
    const requestId = crypto.randomUUID();
    activeRequestId.current = requestId;
    dispatchPanel({ type: 'start', requestId });
    startStreamTimeout(requestId);

    try {
      await runAiAction({ requestId, action, text });
    } catch (err) {
      if (activeRequestId.current !== requestId) return;
      activeRequestId.current = null;
      clearStreamTimeout();
      const message = formatCommandError(err);
      setError(message);
      dispatchPanel({ type: 'reset' });
    }
  }

  function selectAction(action: UiAction) {
    setActiveAction(action);
    setError(null);
    if (!selectedText.trim()) {
      void loadLatestPanelContext(action);
    }
  }

  async function executeActiveAction() {
    let action = activeAction;
    let text = selectedText;

    if (selectedSelectionId) {
      const latest = await loadLatestPanelContextIfNewer(selectedSelectionId);
      if (latest) {
        action = latest.action;
        text = latest.text;
      }
    }

    if (!text.trim()) {
      const latest = await loadLatestPanelContext();
      if (latest) {
        action = latest.action;
        text = latest.text;
      }
    }

    await runActionForText(action, text, { showMissingTextError: true });
  }

  async function loadLatestPanelContextIfNewer(currentSelectionId: string) {
    try {
      const context = await getLatestPanelContext();
      if (!context?.selection?.text?.trim()) return null;
      const latestSelectionId = context.selection.id;
      if (!latestSelectionId || latestSelectionId === currentSelectionId) return null;
      return applyPanelContext(context);
    } catch (err) {
      setError(formatCommandError(err));
      return null;
    }
  }

  function sendFollowUp() {
    setError('追问暂未支持。');
  }

  async function closePanel() {
    try {
      await hideAiPanel();
    } catch (err) {
      setError(formatCommandError(err));
    }
  }

  async function dragPanelFromHeader(event: MouseEvent<HTMLElement>) {
    if ((event.target as Element | null)?.closest(INTERACTIVE_HEADER_SELECTOR)) return;
    try {
      await startDragAiPanel();
    } catch (err) {
      setError(formatCommandError(err));
    }
  }

  return (
    <section className="ai-panel">
      <header className="panel-header" title="拖拽移动面板" onMouseDown={dragPanelFromHeader}>
        <strong>动作：{actionLabels[activeAction]}</strong>
        <button type="button" aria-label="关闭面板" onClick={closePanel}>
          ×
        </button>
      </header>
      <p className="selected-text-preview">{selectedText || '等待选中文本'}</p>
      <ActionBar activeAction={activeAction} onSelect={selectAction} />
      <button type="button" className="execute-action-button" onClick={executeActiveAction}>
        执行当前动作
      </button>
      {error && <p role="alert">{error}</p>}
      <article className="ai-answer" aria-live="polite">
        {panel.answer || (panel.running ? '生成中…' : '点击“执行当前动作”开始生成。')}
      </article>
      <footer className="panel-footer">
        <input value={question} onChange={(event) => setQuestion(event.target.value)} placeholder="追问" />
        <button type="button" onClick={sendFollowUp}>
          发送
        </button>
      </footer>
    </section>
  );
}
