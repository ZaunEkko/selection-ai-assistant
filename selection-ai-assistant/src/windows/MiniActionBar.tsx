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

const AI_STREAM_TIMEOUT_MS = 45_000;
const FALLBACK_TRANSLATE_RESULT_POSITION: Point = { x: 0, y: 0 };

function isValidRect(rect: Rect): boolean {
  return rect.width > 0 && rect.height > 0;
}

function firstValidRect(rects: Rect[]): Rect | null {
  return rects.find(isValidRect) ?? null;
}

function rectContainsPoint(rect: Rect, point: Point): boolean {
  return point.x >= rect.x && point.x <= rect.x + rect.width && point.y >= rect.y && point.y <= rect.y + rect.height;
}

function looksLikeTextSpace(rect: Rect, point?: Point | null): boolean {
  return rect.width >= 240 && rect.height >= 48 && (!point || rectContainsPoint(rect, point));
}

function selectionAnchorPoint(selection: PanelContext['selection']): Point {
  const firstSelectionRect = firstValidRect(selection.selectionRects ?? []);
  const fallbackAnchor = selection.fallbackPoint ?? selection.explicitAnchor;

  if (firstSelectionRect && !looksLikeTextSpace(firstSelectionRect, fallbackAnchor)) {
    return {
      x: firstSelectionRect.x,
      y: firstSelectionRect.y,
    };
  }

  if (firstSelectionRect && looksLikeTextSpace(firstSelectionRect, fallbackAnchor) && selection.fallbackPoint) {
    return selection.fallbackPoint;
  }

  if (selection.explicitAnchor) return selection.explicitAnchor;

  if (selection.fallbackPoint) return selection.fallbackPoint;

  if (selection.anchorRect && isValidRect(selection.anchorRect)) {
    return {
      x: selection.anchorRect.x,
      y: selection.anchorRect.y,
    };
  }

  return FALLBACK_TRANSLATE_RESULT_POSITION;
}

function selectionPlacementRects(selection: PanelContext['selection']): Rect[] {
  const fallbackAnchor = selection.fallbackPoint ?? selection.explicitAnchor;
  const selectionRects = (selection.selectionRects ?? []).filter(
    (rect) => isValidRect(rect) && !looksLikeTextSpace(rect, fallbackAnchor),
  );
  if (selectionRects.length > 0) return selectionRects;
  if (selection.anchorRect && isValidRect(selection.anchorRect) && !looksLikeTextSpace(selection.anchorRect, fallbackAnchor)) {
    return [selection.anchorRect];
  }
  return [];
}

async function collectAiStream(request: { requestId: string; action: UiAction; text: string }): Promise<string> {
  let streamedText = '';
  let finishStream: (() => void) | null = null;
  let failStream: ((error: Error) => void) | null = null;
  const done = new Promise<void>((resolve, reject) => {
    finishStream = resolve;
    failStream = reject;
  });
  let timeoutId: number | undefined;
  const timeout = new Promise<void>((_resolve, reject) => {
    timeoutId = window.setTimeout(() => {
      reject(new Error('AI 服务商响应超时，请稍后重试。'));
    }, AI_STREAM_TIMEOUT_MS);
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
    await Promise.race([done, timeout]);
    return streamedText;
  } finally {
    if (timeoutId !== undefined) window.clearTimeout(timeoutId);
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
    let anchor: Point | null = null;
    let selectionRects: Rect[] = [];
    let originalText = '';

    try {
      const context = await getLatestPanelContext();
      if (!context?.selection?.text) {
        console.error('没有选区文本');
        return;
      }

      anchor = selectionAnchorPoint(context.selection);
      selectionRects = selectionPlacementRects(context.selection);
      originalText = context.selection.text;
      await showTranslateResult(anchor, originalText, '', selectionRects);

      const translatedText = await collectAiStream({
        requestId: `translate-${Date.now()}`,
        action: 'translateOnly',
        text: originalText,
      });
      await showTranslateResult(anchor, originalText, translatedText, selectionRects);
    } catch (err) {
      const message = formatCommandError(err);
      console.error('翻译失败:', message);
      if (anchor && originalText) {
        await showTranslateResult(anchor, originalText, `翻译失败：${message}`, selectionRects);
      }
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
          <span className="mini-action-label">{isReplacing ? '翻译中…' : '替换'}</span>
        </button>
        <button
          className="mini-action-button mini-action-button--translate"
          type="button"
          onClick={() => void handleTranslate()}
          disabled={isTranslating}
          aria-label="翻译文本"
        >
          <span className="mini-action-label">{isTranslating ? '翻译中…' : '翻译'}</span>
        </button>
        <button
          className="mini-action-button mini-action-button--more"
          type="button"
          onClick={() => void handleMore()}
          aria-label="更多操作"
        >
          <span className="mini-action-label">更多</span>
        </button>
      </div>
    </div>
  );
}
