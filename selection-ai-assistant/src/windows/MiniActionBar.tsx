import { listen, emit } from '@tauri-apps/api/event';
import { useEffect, useRef, useState, type MouseEvent } from 'react';
import {
  formatCommandError,
  getConfig,
  getLatestPanelContext,
  hideReplacementPresetPanel,
  openPanelFromFloatingButton,
  replaceSelectedText,
  runAiAction,
  showReplacementPresetPanel,
  showTranslateResult,
  type AppBehaviorConfig,
  type OutputTargetPreset,
  type PanelContext,
  type TargetPresetKind,
  type Point,
  type Rect,
  type UiAction,
} from '../api/tauri';

type StreamDelta = { requestId: string; delta: string };
type StreamDone = { requestId: string };
type StreamError = { requestId: string; code: string; message: string };
type FloatingButtonPointerPosition = { x: number; y: number; width?: number; height?: number };
type ToolbarAction = TargetPresetKind | 'more';

type TargetOption = {
  value: OutputTargetPreset;
  targetLanguage?: string;
};

const AI_STREAM_TIMEOUT_MS = 45_000;
const FALLBACK_TRANSLATE_RESULT_POSITION: Point = { x: 0, y: 0 };
const OUTPUT_TARGET_OPTIONS: TargetOption[] = [
  { value: 'auto' },
  { value: 'chinese', targetLanguage: '中文' },
  { value: 'english', targetLanguage: '英文' },
  { value: 'japanese', targetLanguage: '日文' },
  { value: 'korean', targetLanguage: '韩文' },
  { value: 'classicalChinese', targetLanguage: '文言文' },
  { value: 'oracleBone', targetLanguage: '甲骨文风格近似转写' },
  { value: 'pictograph', targetLanguage: '象形文字风格近似转写' },
  { value: 'morseCode', targetLanguage: '摩斯密码' },
  { value: 'custom' },
];

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

function outputTargetLanguage(behavior: AppBehaviorConfig, kind: TargetPresetKind): string | undefined {
  const targetLanguage = kind === 'translation' ? behavior.translationTargetLanguage : behavior.replacementTargetLanguage;
  const customTarget = kind === 'translation' ? behavior.translationCustomTarget : behavior.replacementCustomTarget;

  if (targetLanguage === 'custom') {
    return customTarget.trim() || undefined;
  }

  return OUTPUT_TARGET_OPTIONS.find((option) => option.value === targetLanguage)?.targetLanguage;
}

async function collectAiStream(request: {
  requestId: string;
  action: UiAction;
  text: string;
  targetLanguage?: string;
  onDelta?: (delta: string) => void;
}): Promise<string> {
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
      request.onDelta?.(event.payload.delta);
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
  const [replaceError, setReplaceError] = useState<string | null>(null);
  const [activeToolbarAction, setActiveToolbarAction] = useState<ToolbarAction | null>(null);
  const presetVisibilityTokenRef = useRef(0);
  const openPresetTimeoutRef = useRef<number | null>(null);
  const pendingPresetKindRef = useRef<TargetPresetKind | null>(null);
  const isPresetPanelOpenRef = useRef(false);
  const currentPresetKindRef = useRef<TargetPresetKind | null>(null);

  function clearPresetTimeout() {
    if (openPresetTimeoutRef.current !== null) {
      window.clearTimeout(openPresetTimeoutRef.current);
      openPresetTimeoutRef.current = null;
    }
    pendingPresetKindRef.current = null;
  }

  async function closeReplacementPresetPanel() {
    clearPresetTimeout();
    presetVisibilityTokenRef.current += 1;
    isPresetPanelOpenRef.current = false;
    currentPresetKindRef.current = null;
    try {
      await hideReplacementPresetPanel();
    } catch (err) {
      console.error('关闭替换目标面板失败:', formatCommandError(err));
    }
  }

  function closeTargetPresetPanelIfVisible() {
    clearPresetTimeout();
    if (!isPresetPanelOpenRef.current && currentPresetKindRef.current === null) return;

    presetVisibilityTokenRef.current += 1;
    isPresetPanelOpenRef.current = false;
    currentPresetKindRef.current = null;
    void hideReplacementPresetPanel().catch((err: unknown) =>
      console.error('关闭替换目标面板失败:', formatCommandError(err)),
    );
  }

  async function handleReplace() {
    if (isReplacing) return;
    setIsReplacing(true);

    try {
      await closeReplacementPresetPanel();
      const context = await getLatestPanelContext();
      if (!context?.selection?.text) {
        console.error('没有选区文本');
        return;
      }

      const config = await getConfig();
      const targetLanguage = outputTargetLanguage(config, 'replacement');
      const translatedText = await collectAiStream({
        requestId: `replace-${Date.now()}`,
        action: 'translateOnly',
        text: context.selection.text,
        ...(targetLanguage ? { targetLanguage } : {}),
      });
      await replaceSelectedText(translatedText, context.selection.id);
    } catch (err) {
      const message = formatCommandError(err);
      console.error('替换失败:', message);
      setReplaceError(message);
      setTimeout(() => setReplaceError(null), 3000);
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
      await closeReplacementPresetPanel();
      const context = await getLatestPanelContext();
      if (!context?.selection?.text) {
        console.error('没有选区文本');
        return;
      }

      anchor = selectionAnchorPoint(context.selection);
      selectionRects = selectionPlacementRects(context.selection);
      originalText = context.selection.text;
      await showTranslateResult(anchor, originalText, '', selectionRects);

      const config = await getConfig();
      const targetLanguage = outputTargetLanguage(config, 'translation');
      const translatedText = await collectAiStream({
        requestId: `translate-${Date.now()}`,
        action: 'translateOnly',
        text: originalText,
        ...(targetLanguage ? { targetLanguage } : {}),
        onDelta: (delta) => {
          void emit('translate_result_delta', { delta });
        },
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
    await closeReplacementPresetPanel();
    await openPanelFromFloatingButton();
  }

  function openTargetPresetPanel(kind: TargetPresetKind) {
    pendingPresetKindRef.current = null;
    const token = presetVisibilityTokenRef.current + 1;
    presetVisibilityTokenRef.current = token;
    isPresetPanelOpenRef.current = true;
    currentPresetKindRef.current = kind;

    void showReplacementPresetPanel(kind)
      .then(() => {
        if (presetVisibilityTokenRef.current === token) return;

        const desiredKind = currentPresetKindRef.current;
        if (!desiredKind) {
          return hideReplacementPresetPanel();
        }
        if (desiredKind !== kind) {
          return showReplacementPresetPanel(desiredKind);
        }
      })
      .catch((err: unknown) => console.error('打开输出目标面板失败:', formatCommandError(err)));
  }

  function handleTargetPresetMouseEnter(kind: TargetPresetKind) {
    if (isPresetPanelOpenRef.current) {
      clearPresetTimeout();
      if (currentPresetKindRef.current !== kind) {
        openTargetPresetPanel(kind);
      }
      return;
    }

    if (pendingPresetKindRef.current === kind) return;

    clearPresetTimeout();
    pendingPresetKindRef.current = kind;
    openPresetTimeoutRef.current = window.setTimeout(() => {
      openTargetPresetPanel(kind);
    }, 150);
  }

  function handleTargetPresetMouseLeave() {
    clearPresetTimeout();
  }

  function handleToolbarActionEnter(action: ToolbarAction) {
    setActiveToolbarAction(action);
    if (action === 'more') {
      closeTargetPresetPanelIfVisible();
      return;
    }

    handleTargetPresetMouseEnter(action);
  }

  function handleToolbarActionLeave(action: ToolbarAction) {
    setActiveToolbarAction((currentAction) => (currentAction === action ? null : currentAction));
    if (action !== 'more') {
      handleTargetPresetMouseLeave();
    }
  }

  function toolbarActionFromValue(value: string | undefined): ToolbarAction | null {
    return value === 'replacement' || value === 'translation' || value === 'more' ? value : null;
  }

  function toolbarActionFromElement(element: Element | null): ToolbarAction | null {
    const actionButton = element?.closest<HTMLElement>('[data-toolbar-action]');
    return toolbarActionFromValue(actionButton?.dataset.toolbarAction);
  }

  function toolbarActionFromPoint(x: number, y: number, scope: ParentNode = document): ToolbarAction | null {
    const elementAction = scope === document ? toolbarActionFromElement(document.elementFromPoint?.(x, y) ?? null) : null;
    if (elementAction) return elementAction;

    const buttons = Array.from(scope.querySelectorAll<HTMLElement>('[data-toolbar-action]'));
    for (const button of buttons) {
      const rect = button.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0 && x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom) {
        return toolbarActionFromValue(button.dataset.toolbarAction);
      }
    }

    return null;
  }

  function toolbarActionFromPointer(event: MouseEvent<HTMLElement>): ToolbarAction | null {
    return toolbarActionFromElement(event.target as Element | null) ?? toolbarActionFromPoint(event.clientX, event.clientY, event.currentTarget);
  }

  function miniActionButtonClass(modifier: string, action: ToolbarAction) {
    return `mini-action-button ${modifier}${activeToolbarAction === action ? ' is-pointer-active' : ''}`;
  }

  function handleToolbarPointerOver(event: MouseEvent<HTMLElement>) {
    const action = toolbarActionFromPointer(event);

    if (action) {
      handleToolbarActionEnter(action);
      return;
    }

    setActiveToolbarAction(null);
    const target = event.target as Element | null;
    if (target?.closest('[data-close-target-preset]')) {
      closeTargetPresetPanelIfVisible();
    }
  }

  function handleToolbarPointerLeave() {
    setActiveToolbarAction(null);
    handleTargetPresetMouseLeave();
  }

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    listen<FloatingButtonPointerPosition>('floating_button_pointer_position', (event) => {
      if (!active) return;
      const x = event.payload.width && event.payload.width > 0 ? (event.payload.x * window.innerWidth) / event.payload.width : event.payload.x;
      const y = event.payload.height && event.payload.height > 0 ? (event.payload.y * window.innerHeight) / event.payload.height : event.payload.y;
      const action = toolbarActionFromPoint(x, y);
      if (action) {
        handleToolbarActionEnter(action);
      }
    })
      .then((nextUnlisten) => {
        if (active) {
          unlisten = nextUnlisten;
        } else {
          nextUnlisten();
        }
      })
      .catch((err: unknown) => console.error('监听迷你条指针位置失败:', formatCommandError(err)));

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  return (
    <div className="mini-action-bar-window">
      <div
        className="mini-action-bar"
        role="toolbar"
        aria-label="文本操作"
        style={{ position: 'relative' }}
        onMouseOver={handleToolbarPointerOver}
        onMouseMove={handleToolbarPointerOver}
        onMouseLeave={handleToolbarPointerLeave}
      >
        <button
          className={miniActionButtonClass('mini-action-button--replace', 'replacement')}
          type="button"
          data-toolbar-action="replacement"
          data-target-preset-kind="replacement"
          onMouseEnter={() => handleToolbarActionEnter('replacement')}
          onMouseMove={() => handleToolbarActionEnter('replacement')}
          onMouseLeave={() => handleToolbarActionLeave('replacement')}
          onFocus={() => handleToolbarActionEnter('replacement')}
          onBlur={() => handleToolbarActionLeave('replacement')}
          onClick={() => void handleReplace()}
          disabled={isReplacing}
          aria-label="翻译并替换文本"
        >
          <span className="mini-action-label">{isReplacing ? '替换中…' : '替换'}</span>
        </button>
        <button
          className={miniActionButtonClass('mini-action-button--translate', 'translation')}
          type="button"
          data-toolbar-action="translation"
          data-target-preset-kind="translation"
          onMouseEnter={() => handleToolbarActionEnter('translation')}
          onMouseMove={() => handleToolbarActionEnter('translation')}
          onMouseLeave={() => handleToolbarActionLeave('translation')}
          onFocus={() => handleToolbarActionEnter('translation')}
          onBlur={() => handleToolbarActionLeave('translation')}
          onClick={() => void handleTranslate()}
          disabled={isTranslating}
          aria-label="翻译文本"
        >
          <span className="mini-action-label">{isTranslating ? '翻译中…' : '翻译'}</span>
        </button>
        <button
          className={miniActionButtonClass('mini-action-button--more', 'more')}
          type="button"
          data-toolbar-action="more"
          data-close-target-preset="true"
          onMouseEnter={() => handleToolbarActionEnter('more')}
          onMouseMove={() => handleToolbarActionEnter('more')}
          onMouseLeave={() => handleToolbarActionLeave('more')}
          onFocus={() => handleToolbarActionEnter('more')}
          onBlur={() => handleToolbarActionLeave('more')}
          onPointerDown={closeTargetPresetPanelIfVisible}
          onClick={() => void handleMore()}
          aria-label="更多操作"
        >
          <span className="mini-action-label">更多</span>
        </button>
        {replaceError && (
          <div className="mini-action-error" style={{ position: 'absolute', top: '100%', left: '50%', transform: 'translateX(-50%)', marginTop: '8px', padding: '6px 12px', background: 'var(--macos-red)', color: 'white', borderRadius: '6px', fontSize: '12px', whiteSpace: 'nowrap', boxShadow: 'var(--macos-shadow-md)', pointerEvents: 'none', zIndex: 10 }}>
            {replaceError}
          </div>
        )}
      </div>
    </div>
  );
}
