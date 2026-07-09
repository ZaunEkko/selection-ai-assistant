import type { MouseEvent } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { cancelScreenshotTranslate, formatCommandError, runScreenshotTranslate, type Point, type Rect } from '../api/tauri';

type DragState = {
  start: Point;
  current: Point;
};

const MIN_CAPTURE_SIZE = 8;
const DEFAULT_MESSAGE = '拖拽框选不可选中的文字区域';

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

export function ScreenshotOverlay() {
  const [drag, setDrag] = useState<DragState | null>(null);
  const [capturing, setCapturing] = useState(false);
  const [message, setMessage] = useState(DEFAULT_MESSAGE);
  const dragRef = useRef<DragState | null>(null);
  const selectionRect = useMemo(() => (drag ? rectFromDrag(drag) : null), [drag]);

  const resetSelection = useCallback((nextMessage = DEFAULT_MESSAGE) => {
    dragRef.current = null;
    setDrag(null);
    setCapturing(false);
    setMessage(nextMessage);
  }, []);

  useEffect(() => {
    dragRef.current = drag;
  }, [drag]);

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

  async function finishSelection() {
    const currentDrag = dragRef.current;
    if (!currentDrag || capturing) return;

    const rect = rectFromDrag(currentDrag);
    if (!isLargeEnough(rect)) {
      resetSelection('截图区域太小，请重新拖拽框选。');
      return;
    }

    setCapturing(true);
    setMessage('正在识别并翻译截图…');
    try {
      await runScreenshotTranslate({
        requestId: newRequestId(),
        rect,
        viewportSize: { width: window.innerWidth, height: window.innerHeight },
      });
      resetSelection();
    } catch (err) {
      resetSelection(formatCommandError(err));
    }
  }

  return (
    <main
      className="screenshot-overlay-window"
      onMouseDown={(event) => {
        if (capturing || event.button !== 0) return;
        const point = pointFromMouseEvent(event);
        const nextDrag = { start: point, current: point };
        dragRef.current = nextDrag;
        setDrag(nextDrag);
        setMessage('松开鼠标后开始截图翻译');
      }}
      onMouseMove={(event) => {
        if (!dragRef.current || capturing) return;
        const nextDrag = { ...dragRef.current, current: pointFromMouseEvent(event) };
        dragRef.current = nextDrag;
        setDrag(nextDrag);
      }}
      onMouseUp={() => void finishSelection()}
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
      <div className="screenshot-overlay-hint">
        <strong>截图翻译</strong>
        <span>{message}</span>
        <kbd>Esc</kbd>
      </div>
    </main>
  );
}
