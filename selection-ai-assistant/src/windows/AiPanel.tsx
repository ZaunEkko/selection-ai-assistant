import { listen } from '@tauri-apps/api/event';
import { useEffect, useReducer, useRef, useState, type KeyboardEvent, type MouseEvent } from 'react';
import {
  formatCommandError,
  getLatestPanelContext,
  hideAiPanel,
  hideSourceTextWindow,
  runAiAction,
  runAiFollowUp,
  showSourceTextWindow,
  startDragAiPanel,
  type PanelContext,
  type UiAction,
} from '../api/tauri';
import { ActionBar } from '../components/ActionBar';
import { actionLabels } from '../components/actionLabels';
import { MarkdownPreview } from '../components/MarkdownPreview';
import { initialPanelState, panelReducer } from '../stores/panelStore';

type StreamDelta = { requestId: string; delta: string };
type StreamError = { requestId: string; code: string; message: string };
type StreamDone = { requestId: string };

const AI_STREAM_TIMEOUT_MS = 60_000;
const INTERACTIVE_HEADER_SELECTOR = 'button, input, textarea, select, a, [role="button"]';
const SELECTED_TEXT_PREVIEW_LIMIT = 140;
const AUTO_SCROLL_BOTTOM_THRESHOLD_PX = 48;

function isLongSelectedText(text: string) {
  return text.length > SELECTED_TEXT_PREVIEW_LIMIT;
}

function selectedTextPreview(text: string, expanded: boolean) {
  if (!text) return '等待选中文本';
  if (expanded || !isLongSelectedText(text)) return text;
  return `${text.slice(0, SELECTED_TEXT_PREVIEW_LIMIT)}…`;
}

function isScrolledNearBottom(element: HTMLElement) {
  const remaining = element.scrollHeight - element.clientHeight - element.scrollTop;
  return remaining <= AUTO_SCROLL_BOTTOM_THRESHOLD_PX;
}

export function AiPanel() {
  const [panel, dispatchPanel] = useReducer(panelReducer, initialPanelState);
  const [activeAction, setActiveAction] = useState<UiAction>('explain');
  const [selectedText, setSelectedText] = useState('');
  const [selectedSelectionId, setSelectedSelectionId] = useState<string | null>(null);
  const [selectedTextExpanded, setSelectedTextExpanded] = useState(false);
  const [sourceWindowOpen, setSourceWindowOpen] = useState(false);
  const [question, setQuestion] = useState('');
  const [error, setError] = useState<string | null>(null);
  const activeRequestId = useRef<string | null>(null);
  const selectedSelectionIdRef = useRef<string | null>(null);
  const selectedTextRef = useRef('');
  const streamTimeoutId = useRef<number | null>(null);
  const answerRef = useRef<HTMLElement | null>(null);
  const shouldAutoScrollAnswer = useRef(true);

  function isSameVisibleSelection(context: PanelContext) {
    const incomingSelectionId = context.selection.id ?? null;
    if (incomingSelectionId && selectedSelectionIdRef.current) {
      return incomingSelectionId === selectedSelectionIdRef.current;
    }
    return context.selection.text.trim() === selectedTextRef.current.trim();
  }

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;

    listen<PanelContext>('panel_context', (event) => {
      if (activeRequestId.current !== null) {
        if (event.payload.autoRun === true || isSameVisibleSelection(event.payload)) return;
        syncPanelSelection(event.payload);
        return;
      }

      const { action, text } = applyPanelContext(event.payload);
      if (event.payload.autoRun === true) {
        void runActionForText(action, text);
      }
    }).then((fn) => {
      if (disposed) fn();
      else unlisteners.push(fn);
    });

    listen('source_text_window_hidden', () => {
      setSourceWindowOpen(false);
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

  useEffect(() => {
    const answer = answerRef.current;
    if (!answer || !shouldAutoScrollAnswer.current) return;
    answer.scrollTop = answer.scrollHeight;
  }, [panel.answer]);

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

  function syncPanelSelection(context: PanelContext, actionOverride?: UiAction) {
    const text = context.selection.text;
    const action = actionOverride ?? context.action;
    setSelectedText(text);
    setSelectedSelectionId(context.selection.id ?? null);
    selectedTextRef.current = text;
    selectedSelectionIdRef.current = context.selection.id ?? null;
    setSelectedTextExpanded(false);
    setSourceWindowOpen(false);
    setActiveAction(action);
    setError(null);
    return { action, text, selectionId: context.selection.id ?? null };
  }

  function applyPanelContext(context: PanelContext, actionOverride?: UiAction) {
    const result = syncPanelSelection(context, actionOverride);
    activeRequestId.current = null;
    shouldAutoScrollAnswer.current = true;
    clearStreamTimeout();
    dispatchPanel({ type: 'reset' });
    return result;
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
    shouldAutoScrollAnswer.current = true;
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

  async function sendFollowUp() {
    const trimmedQuestion = question.trim();
    const previousAnswer = panel.answer.trim();

    if (!trimmedQuestion) {
      setError('请输入追问内容。');
      return;
    }
    if (!selectedText.trim()) {
      setError('请先选中文本后再追问。');
      return;
    }
    if (!previousAnswer) {
      setError('请先执行一次 AI 动作后再追问。');
      return;
    }

    setError(null);
    setQuestion('');
    shouldAutoScrollAnswer.current = true;
    const requestId = crypto.randomUUID();
    const initialAnswer = `${previousAnswer}\n\n---\n\n追问：${trimmedQuestion}\n\n回答：`;
    activeRequestId.current = requestId;
    dispatchPanel({ type: 'start', requestId, initialAnswer });
    startStreamTimeout(requestId);

    try {
      await runAiFollowUp({
        requestId,
        originalText: selectedText,
        previousAnswer,
        question: trimmedQuestion,
      });
    } catch (err) {
      if (activeRequestId.current !== requestId) return;
      activeRequestId.current = null;
      clearStreamTimeout();
      setError(formatCommandError(err));
      dispatchPanel({ type: 'finish', requestId });
    }
  }

  function sendFollowUpOnEnter(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== 'Enter' || event.shiftKey) return;
    event.preventDefault();
    void sendFollowUp();
  }

  async function openSourceWindow() {
    if (sourceWindowOpen) {
      try {
        await hideSourceTextWindow();
        setSourceWindowOpen(false);
      } catch (err) {
        setError(formatCommandError(err));
      }
      return;
    }

    let text = selectedText;

    if (selectedSelectionId) {
      const latest = await loadLatestPanelContextIfNewer(selectedSelectionId);
      if (latest) {
        text = latest.text;
      }
    }

    if (!text.trim()) {
      const latest = await loadLatestPanelContext();
      if (latest) {
        text = latest.text;
      }
    }

    if (!text.trim()) return;

    try {
      await showSourceTextWindow(text);
      setSourceWindowOpen(true);
    } catch (err) {
      setError(formatCommandError(err));
    }
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

  function updateAnswerAutoScrollPreference() {
    const answer = answerRef.current;
    if (!answer) return;
    shouldAutoScrollAnswer.current = isScrolledNearBottom(answer);
  }

  const hasSelectedText = selectedText.trim().length > 0;
  const selectedTextIsLong = isLongSelectedText(selectedText);
  const topPreviewText = selectedTextPreview(selectedText, selectedTextExpanded);

  return (
    <section className="ai-panel">
      <header className="panel-header" title="拖拽移动面板" onMouseDown={dragPanelFromHeader}>
        <strong>动作：{actionLabels[activeAction]}</strong>
        <button type="button" aria-label="关闭面板" onClick={closePanel}>
          ×
        </button>
      </header>

      <section className="selected-text-card" aria-label="选中文本">
        <div className="selected-text-card-header">
          <strong>选中文本</strong>
          {hasSelectedText && (
            <button type="button" className="source-window-button" aria-pressed={sourceWindowOpen} onClick={openSourceWindow}>
              {sourceWindowOpen ? '关闭左侧原文' : '在左侧窗口打开原文'}
            </button>
          )}
        </div>
        <p className={`selected-text-preview${selectedTextIsLong ? (selectedTextExpanded ? ' is-expanded' : ' is-collapsed') : ''}`}>
          {topPreviewText}
        </p>
        {selectedTextIsLong && (
          <div className="selected-text-actions">
            <span>{selectedTextExpanded ? '完整原文已展开' : '已默认缩略，避免挤占回答区域'}</span>
            <button
              type="button"
              className="selected-text-expand-button"
              aria-expanded={selectedTextExpanded}
              onClick={() => setSelectedTextExpanded((expanded) => !expanded)}
            >
              {selectedTextExpanded ? '收起原文' : '显示完整原文'}
            </button>
          </div>
        )}
      </section>

      <div className="panel-controls">
        <ActionBar activeAction={activeAction} onSelect={selectAction} />
        <div className="panel-control-buttons">
          <button type="button" className="execute-action-button" onClick={executeActiveAction}>
            执行当前动作
          </button>
        </div>
        {error && <p role="alert">{error}</p>}
      </div>

      <div className="panel-body">
        <article ref={answerRef} className="ai-answer" aria-live="polite" onScroll={updateAnswerAutoScrollPreference}>
          {panel.answer ? <MarkdownPreview markdown={panel.answer} /> : panel.running ? '生成中…' : '点击“执行当前动作”开始生成。'}
        </article>
      </div>

      <footer className="panel-footer">
        <textarea
          value={question}
          onChange={(event) => setQuestion(event.target.value)}
          onKeyDown={sendFollowUpOnEnter}
          placeholder="追问"
          rows={1}
          aria-label="追问"
        />
        <button type="button" onClick={sendFollowUp}>
          发送
        </button>
      </footer>
    </section>
  );
}
