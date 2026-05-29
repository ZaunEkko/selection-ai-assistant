import { listen } from '@tauri-apps/api/event';
import { useEffect, useReducer, useState } from 'react';
import { formatCommandError, runAiAction, type UiAction } from '../api/tauri';
import { ActionBar } from '../components/ActionBar';
import { initialPanelState, panelReducer } from '../stores/panelStore';

type PanelContext = {
  selection: { text: string; sourceApp: string; windowTitle: string };
  action: UiAction;
};

type StreamDelta = { requestId: string; delta: string };
type StreamDone = { requestId: string };

export function AiPanel() {
  const [panel, dispatchPanel] = useReducer(panelReducer, initialPanelState);
  const [activeAction, setActiveAction] = useState<UiAction>('explain');
  const [selectedText, setSelectedText] = useState('');
  const [question, setQuestion] = useState('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;

    listen<PanelContext>('panel_context', (event) => {
      setSelectedText(event.payload.selection.text);
      setActiveAction(event.payload.action);
      setError(null);
      dispatchPanel({ type: 'reset' });
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

  async function runAction(action: UiAction) {
    if (!selectedText.trim()) return;

    setActiveAction(action);
    setError(null);
    const requestId = crypto.randomUUID();
    dispatchPanel({ type: 'start', requestId });

    try {
      await runAiAction({ requestId, action, text: selectedText });
    } catch (err) {
      const message = formatCommandError(err);
      setError(message);
      dispatchPanel({ type: 'reset' });
    }
  }

  return (
    <section className="ai-panel">
      <header className="panel-header">
        <strong>动作：{activeAction}</strong>
        <button type="button" aria-label="Close panel">
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
