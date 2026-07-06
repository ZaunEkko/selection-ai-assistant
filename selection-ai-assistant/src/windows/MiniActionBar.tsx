import { listen } from '@tauri-apps/api/event';
import { useState } from 'react';
import {
  formatCommandError,
  getLatestPanelContext,
  openPanelFromFloatingButton,
  replaceSelectedText,
  runAiAction,
  showTranslateResult,
  type PanelContext,
  type Point,
  type Rect,
  type UiAction,
} from '../api/tauri';

type StreamDelta = { requestId: string; delta: string };
type StreamDone = { requestId: string };
type StreamError = { requestId: string; code: string; message: string };

const FALLBACK_TRANSLATE_RESULT_POSITION: Point = { x: 0, y: 0 };

function isValidRect(rect: Rect): boolean {
  return rect.width > 0 && rect.height > 0;
}

function firstValidRect(rects: Rect[]): Rect | null {
  return rects.find(isValidRect) ?? null;
}

function selectionAnchorPoint(selection: PanelContext['selection']): Point {
  const firstSelectionRect = firstValidRect(selection.selectionRects ?? []);
  if (firstSelectionRect) {
    return {
      x: firstSelectionRect.x,
      y: firstSelectionRect.y,
    };
  }

  if (selection.explicitAnchor) return selection.explicitAnchor;

  if (selection.anchorRect && isValidRect(selection.anchorRect)) {
    return {
      x: selection.anchorRect.x,
      y: selection.anchorRect.y,
    };
  }

  return selection.fallbackPoint ?? FALLBACK_TRANSLATE_RESULT_POSITION;
}

async function collectAiStream(request: { requestId: string; action: UiAction; text: string }): Promise<string> {
  let streamedText = '';
  let finishStream: (() => void) | null = null;
  let failStream: ((error: Error) => void) | null = null;
  const done = new Promise<void>((resolve, reject) => {
    finishStream = resolve;
    failStream = reject;
  });

  const unlistenDelta = await listen<StreamDelta>('ai_stream_delta', (event) => {
    if (event.payload.requestId === request.requestId) {
      streamedText += event.payload.delta;
    }
  });
  const unlistenError = await listen<StreamError>('ai_stream_error', (event) => {
    if (event.payload.requestId === request.requestId) {
      failStream?.(new Error(event.payload.message || event.payload.code));
    }
  });
  const unlistenDone = await listen<StreamDone>('ai_stream_done', (event) => {
    if (event.payload.requestId === request.requestId) {
      finishStream?.();
    }
  });

  try {
    await runAiAction(request);
    await done;
    return streamedText;
  } finally {
    unlistenDelta();
    unlistenError();
    unlistenDone();
  }
}

export function MiniActionBar() {
  const [isTranslating, setIsTranslating] = useState(false);
  const [isReplacing, setIsReplacing] = useState(false);

  async function handleReplace() {
    if (isReplacing) return;
    setIsReplacing(true);

    try {
      const context = await getLatestPanelContext();
      if (!context?.selection?.text) {
        console.error('没有选区文本');
        return;
      }

      const translatedText = await collectAiStream({
        requestId: `replace-${Date.now()}`,
        action: 'translateOnly',
        text: context.selection.text,
      });
      await replaceSelectedText(translatedText, context.selection.id);
    } catch (err) {
      console.error('替换失败:', formatCommandError(err));
    } finally {
      setIsReplacing(false);
    }
  }

  async function handleTranslate() {
    if (isTranslating) return;
    setIsTranslating(true);

    try {
      const context = await getLatestPanelContext();
      if (!context?.selection?.text) {
        console.error('没有选区文本');
        return;
      }

      const translatedText = await collectAiStream({
        requestId: `translate-${Date.now()}`,
        action: 'translateExplain',
        text: context.selection.text,
      });
      await showTranslateResult(selectionAnchorPoint(context.selection), context.selection.text, translatedText);
    } catch (err) {
      console.error('翻译失败:', formatCommandError(err));
    } finally {
      setIsTranslating(false);
    }
  }

  async function handleMore() {
    await openPanelFromFloatingButton();
  }

  return (
    <div className="mini-action-bar-window">
      <div className="mini-action-bar" role="toolbar" aria-label="文本操作">
        <button
          className="mini-action-button mini-action-button--replace"
          type="button"
          onClick={() => void handleReplace()}
          disabled={isReplacing}
          aria-label="翻译并替换文本"
        >
          <span className="mini-action-icon" aria-hidden="true">🔄</span>
          <span className="mini-action-label">{isReplacing ? '翻译中…' : '替换'}</span>
        </button>
        <button
          className="mini-action-button mini-action-button--translate"
          type="button"
          onClick={() => void handleTranslate()}
          disabled={isTranslating}
          aria-label="翻译文本"
        >
          <span className="mini-action-icon" aria-hidden="true">📖</span>
          <span className="mini-action-label">{isTranslating ? '翻译中…' : '翻译'}</span>
        </button>
        <button
          className="mini-action-button mini-action-button--more"
          type="button"
          onClick={() => void handleMore()}
          aria-label="更多操作"
        >
          <span className="mini-action-icon" aria-hidden="true">⋯</span>
          <span className="mini-action-label">更多</span>
        </button>
      </div>
    </div>
  );
}
