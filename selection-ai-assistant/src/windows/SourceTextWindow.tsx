import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useState, type MouseEvent } from 'react';
import {
  formatCommandError,
  getLatestSourceTextContext,
  hideSourceTextWindow,
  startDragSourceTextWindow,
  type PanelContext,
  type SourceTextContext,
} from '../api/tauri';

const INTERACTIVE_HEADER_SELECTOR = 'button, input, textarea, select, a, [role="button"]';

export function SourceTextWindow() {
  const [text, setText] = useState('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void | Promise<void>> = [];

    function applySourceText(nextText: string) {
      setText(nextText);
      setError(null);
    }

    async function recoverLatestSourceText(replaceExisting: boolean) {
      try {
        const context = await getLatestSourceTextContext();
        if (disposed || !context?.text?.trim()) return;
        if (replaceExisting) {
          applySourceText(context.text);
        } else {
          setText((current) => (current.trim() ? current : context.text));
          setError(null);
        }
      } catch (err) {
        if (!disposed) setError(formatCommandError(err));
      }
    }

    listen<SourceTextContext>('source_text_context', (event) => {
      applySourceText(event.payload.text);
    }).then((fn) => {
      if (disposed) fn();
      else unlisteners.push(fn);
    });

    listen<PanelContext>('panel_context', (event) => {
      applySourceText(event.payload.selection.text);
    }).then((fn) => {
      if (disposed) fn();
      else unlisteners.push(fn);
    });

    getCurrentWindow()
      .onFocusChanged((event) => {
        if (event.payload) void recoverLatestSourceText(true);
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisteners.push(fn);
      });

    void recoverLatestSourceText(false);

    return () => {
      disposed = true;
      unlisteners.forEach((fn) => void fn());
    };
  }, []);

  async function closeWindow() {
    try {
      await hideSourceTextWindow();
    } catch (err) {
      setError(formatCommandError(err));
    }
  }

  async function dragWindowFromHeader(event: MouseEvent<HTMLElement>) {
    if ((event.target as Element | null)?.closest(INTERACTIVE_HEADER_SELECTOR)) return;
    try {
      await startDragSourceTextWindow();
    } catch (err) {
      setError(formatCommandError(err));
    }
  }

  return (
    <section className="source-text-window">
      <header className="source-text-header" title="拖拽移动原文窗口" onMouseDown={dragWindowFromHeader}>
        <h1>原文</h1>
        <button type="button" aria-label="关闭原文窗口" onClick={closeWindow}>
          ×
        </button>
      </header>
      {error && <p role="alert">{error}</p>}
      <article className="source-text-body">{text || '等待原文内容'}</article>
    </section>
  );
}
