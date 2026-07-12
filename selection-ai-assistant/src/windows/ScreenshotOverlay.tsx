import type { MouseEvent } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  cancelScreenshotTranslate,
  formatCommandError,
  getConfig,
  runScreenshotTranslate,
  type OutputTargetPreset,
  type Point,
  type Rect,
} from '../api/tauri';

type DragState = {
  start: Point;
  current: Point;
};

type CapturePhase = 'idle' | 'dragging' | 'pending' | 'capturing';

type TargetOption = {
  value: OutputTargetPreset;
  label: string;
  targetLanguage?: string;
};

const MIN_CAPTURE_SIZE = 8;
const DEFAULT_MESSAGE = '拖拽框选不可选中的文字区域';
const CONFIRM_CONTROLS_SIZE = { width: 328, height: 82 };
const CONFIRM_CONTROLS_GAP = 10;
const SCREENSHOT_TARGET_OPTIONS: TargetOption[] = [
  { value: 'auto', label: '自动' },
  { value: 'chinese', label: '中文', targetLanguage: '中文' },
  { value: 'english', label: '英文', targetLanguage: '英文' },
  { value: 'japanese', label: '日文', targetLanguage: '日文' },
  { value: 'korean', label: '韩文', targetLanguage: '韩文' },
  { value: 'classicalChinese', label: '文言', targetLanguage: '文言文' },
  { value: 'oracleBone', label: '甲骨', targetLanguage: '甲骨文风格近似转写' },
  { value: 'pictograph', label: '象形', targetLanguage: '象形文字风格近似转写' },
  { value: 'morseCode', label: '摩斯', targetLanguage: '摩斯密码' },
  { value: 'custom', label: '自定' },
];

function pointFromMouseEvent(event: MouseEvent): Point {
  return {
    x: event.clientX,
    y: event.clientY,
  };
}

function rectFromDrag(drag: DragState): Rect {
  const left = Math.min(drag.start.x, drag.current.x);
  const top = Math.min(drag.start.y, drag.current.y);
  return {
    x: left,
    y: top,
    width: Math.abs(drag.current.x - drag.start.x),
    height: Math.abs(drag.current.y - drag.start.y),
  };
}

function isLargeEnough(rect: Rect) {
  return rect.width >= MIN_CAPTURE_SIZE && rect.height >= MIN_CAPTURE_SIZE;
}

function newRequestId() {
  return `screenshot-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}`}`;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function confirmControlsPosition(rect: Rect): Point {
  const x = rect.x + rect.width - CONFIRM_CONTROLS_SIZE.width;
  const belowY = rect.y + rect.height + CONFIRM_CONTROLS_GAP;
  const aboveY = rect.y - CONFIRM_CONTROLS_SIZE.height - CONFIRM_CONTROLS_GAP;
  const maxX = window.innerWidth - CONFIRM_CONTROLS_SIZE.width - CONFIRM_CONTROLS_GAP;
  const maxY = window.innerHeight - CONFIRM_CONTROLS_SIZE.height - CONFIRM_CONTROLS_GAP;
  const y = belowY + CONFIRM_CONTROLS_SIZE.height <= window.innerHeight - CONFIRM_CONTROLS_GAP ? belowY : aboveY;

  return {
    x: clamp(x, CONFIRM_CONTROLS_GAP, maxX),
    y: clamp(y, CONFIRM_CONTROLS_GAP, maxY),
  };
}

function targetLanguageForPreset(targetPreset: OutputTargetPreset, customTarget: string) {
  if (targetPreset === 'custom') return customTarget.trim() || undefined;
  return SCREENSHOT_TARGET_OPTIONS.find((option) => option.value === targetPreset)?.targetLanguage;
}

export function ScreenshotOverlay() {
  const [drag, setDrag] = useState<DragState | null>(null);
  const [phase, setPhaseState] = useState<CapturePhase>('idle');
  const [message, setMessage] = useState(DEFAULT_MESSAGE);
  const [targetPreset, setTargetPreset] = useState<OutputTargetPreset>('auto');
  const [customTarget, setCustomTarget] = useState('');
  const phaseRef = useRef<CapturePhase>('idle');
  const dragRef = useRef<DragState | null>(null);
  const selectionRect = useMemo(() => (drag ? rectFromDrag(drag) : null), [drag]);
  const controlsPosition = useMemo(
    () => (selectionRect && phase === 'pending' ? confirmControlsPosition(selectionRect) : null),
    [phase, selectionRect],
  );

  const setPhase = useCallback((nextPhase: CapturePhase) => {
    phaseRef.current = nextPhase;
    setPhaseState(nextPhase);
  }, []);

  const resetSelection = useCallback(
    (nextMessage = DEFAULT_MESSAGE) => {
      dragRef.current = null;
      setDrag(null);
      setPhase('idle');
      setMessage(nextMessage);
    },
    [setPhase],
  );

  useEffect(() => {
    dragRef.current = drag;
  }, [drag]);

  useEffect(() => {
    let active = true;
    getConfig()
      .then((config) => {
        if (!active) return;
        setTargetPreset(config.translationTargetLanguage);
        setCustomTarget(config.translationCustomTarget);
      })
      .catch((err) => setMessage(formatCommandError(err)));

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        event.preventDefault();
        resetSelection();
        void cancelScreenshotTranslate();
      }
    }

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [resetSelection]);

  function finishSelection() {
    const currentDrag = dragRef.current;
    if (!currentDrag || phaseRef.current !== 'dragging') return;

    const rect = rectFromDrag(currentDrag);
    if (!isLargeEnough(rect)) {
      resetSelection('截图区域太小，请重新拖拽框选。');
      return;
    }

    setPhase('pending');
    setMessage('确认后开始识别并翻译截图');
  }

  async function confirmSelection() {
    const currentDrag = dragRef.current;
    if (!currentDrag || phaseRef.current !== 'pending') return;

    const rect = rectFromDrag(currentDrag);
    const targetLanguage = targetLanguageForPreset(targetPreset, customTarget);
    if (targetPreset === 'custom' && !targetLanguage) {
      setMessage('请输入自定义截图翻译目标后再确认。');
      return;
    }

    setPhase('capturing');
    setMessage('正在识别并翻译截图…');
    try {
      await runScreenshotTranslate({
        requestId: newRequestId(),
        rect,
        viewportSize: { width: window.innerWidth, height: window.innerHeight },
        ...(targetLanguage ? { targetLanguage } : {}),
      });
      resetSelection();
    } catch (err) {
      resetSelection(formatCommandError(err));
    }
  }

  async function cancelSelection() {
    resetSelection();
    await cancelScreenshotTranslate();
  }

  return (
    <main
      className={`screenshot-overlay-window is-${phase}`}
      onMouseDown={(event) => {
        if (phaseRef.current === 'capturing' || event.button !== 0) return;
        const point = pointFromMouseEvent(event);
        const nextDrag = { start: point, current: point };
        dragRef.current = nextDrag;
        setDrag(nextDrag);
        setPhase('dragging');
        setMessage('松开鼠标后确认截图翻译');
      }}
      onMouseMove={(event) => {
        if (!dragRef.current || phaseRef.current !== 'dragging') return;
        const nextDrag = { ...dragRef.current, current: pointFromMouseEvent(event) };
        dragRef.current = nextDrag;
        setDrag(nextDrag);
      }}
      onMouseUp={finishSelection}
      role="application"
      aria-label="截图翻译取景层"
    >
      <div className="screenshot-overlay-backdrop" />
      {selectionRect ? (
        <div
          className="screenshot-selection-rect"
          style={{
            left: `${selectionRect.x}px`,
            top: `${selectionRect.y}px`,
            width: `${selectionRect.width}px`,
            height: `${selectionRect.height}px`,
          }}
        />
      ) : null}
      {controlsPosition ? (
        <div
          className="screenshot-confirm-controls"
          style={{ left: `${controlsPosition.x}px`, top: `${controlsPosition.y}px` }}
          onPointerDown={(event) => event.stopPropagation()}
          onPointerUp={(event) => event.stopPropagation()}
          onMouseDown={(event) => event.stopPropagation()}
          onMouseUp={(event) => event.stopPropagation()}
          onClick={(event) => event.stopPropagation()}
        >
          <label className="screenshot-target-select">
            <span>翻译为</span>
            <select
              value={targetPreset}
              aria-label="截图翻译目标"
              onChange={(event) => setTargetPreset(event.target.value as OutputTargetPreset)}
            >
              {SCREENSHOT_TARGET_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          {targetPreset === 'custom' ? (
            <input
              className="screenshot-custom-target"
              value={customTarget}
              aria-label="自定义截图翻译目标"
              placeholder="摩斯密码 / 象形文字"
              onChange={(event) => setCustomTarget(event.target.value)}
            />
          ) : null}
          <div className="screenshot-confirm-actions">
            <button type="button" className="screenshot-cancel-button" aria-label="取消本次截图翻译" onClick={() => void cancelSelection()}>
              ×
            </button>
            <button type="button" className="screenshot-confirm-button" aria-label="确认本次截图翻译" onClick={() => void confirmSelection()}>
              ✓
            </button>
          </div>
        </div>
      ) : null}
      <div className="screenshot-overlay-hint">
        <strong>截图翻译</strong>
        <span>{message}</span>
        <kbd>Esc</kbd>
      </div>
    </main>
  );
}
