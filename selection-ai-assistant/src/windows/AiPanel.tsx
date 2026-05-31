import { listen } from '@tauri-apps/api/event';
import { useEffect, useReducer, useRef, useState } from 'react';
import { formatCommandError, hideAiPanel, runAiAction, type UiAction } from '../api/tauri';
import { ActionBar } from '../components/ActionBar';
import { initialPanelState, panelReducer } from '../stores/panelStore';

type PanelContext = {
  selection: { text: string; sourceApp: string; windowTitle: string };
  action: UiAction;
  autoRun?: boolean;
};

type StreamDelta = { requestId: string; delta: string };
type StreamDone = { requestId: string };

export function AiPanel() {
  const [panel, dispatchPanel] = useReducer(panelReducer, initialPanelState);
  const [activeAction, setActiveAction] = useState<UiAction>('explain');
  const [selectedText, setSelectedText] = useState('');
  const [question, setQuestion] = useState('');
  const [error, setError] = useState<string | null>(null);
  const activeRequestId = useRef<string | null>(null);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;

    listen<PanelContext>('panel_context', (event) => {
      const text = event.payload.selection.text;
      setSelectedText(text);
      setActiveAction(event.payload.action);
      setError(null);
      activeRequestId.current = null;
      dispatchPanel({ type: 'reset' });
      if (event.payload.autoRun === true) {
        void runActionForText(event.payload.action, text);
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

    listen<StreamDone>('ai_stream_done', (event) => {
      if (activeRequestId.current === event.payload.requestId) {
        activeRequestId.current = null;
      }
      dispatchPanel({ type: 'finish', requestId: event.payload.requestId });
    }).then((fn) => {
      if (disposed) fn();
      else unlisteners.push(fn);
    });

    return () => {
      disposed = true;
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  async function runActionForText(action: UiAction, text: string) {
    if (!text.trim()) return;

    setActiveAction(action);
    setError(null);
    const requestId = crypto.randomUUID();
    activeRequestId.current = requestId;
    dispatchPanel({ type: 'start', requestId });

    try {
      await runAiAction({ requestId, action, text });
    } catch (err) {
      if (activeRequestId.current !== requestId) return;
      activeRequestId.current = null;
      const message = formatCommandError(err);
      setError(message);
      dispatchPanel({ type: 'reset' });
    }
  }

  async function runAction(action: UiAction) {
    await runActionForText(action, selectedText);
  }

  async function closePanel() {
    try {
      await hideAiPanel();
    } catch (err) {
      setError(formatCommandError(err));
    }
  }

  return (
    <section className="ai-panel">
      <header className="panel-header">
        <strong>动作：{activeAction}</strong>
        <button type="button" aria-label="Close panel" onClick={closePanel}>
          ×
        </button>
      </header>
      <p className="selected-text-preview">{selectedText || '等待选中文本'}</p>
      <ActionBar activeAction={activeAction} onRun={runAction} />
      {error && <p role="alert">{error}</p>}
      <article className="ai-answer" aria-live="polite">
        {panel.answer || (panel.running ? '生成中…' : '点击动作开始生成。')}
      </article>
      <footer className="panel-footer">
        <input value={question} onChange={(event) => setQuestion(event.target.value)} placeholder="追问" />
        <button type="button" onClick={() => runAction(activeAction)}>
          发送
        </button>
      </footer>
    </section>
  );
}
