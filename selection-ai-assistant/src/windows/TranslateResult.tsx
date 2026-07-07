import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import {
  formatCommandError,
  hideTranslateResult,
  startDragTranslateResultWindow,
  startResizeTranslateResultWindow,
} from '../api/tauri';

type TranslateResultEvent = {
  originalText: string;
  translatedText: string;
};

export function TranslateResult() {
  const [translatedText, setTranslatedText] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    listen<TranslateResultEvent>('translate_result', (event) => {
      if (active) {
        setTranslatedText(event.payload.translatedText);
        setError(null);
      }
    })
      .then((nextUnlisten) => {
        if (active) {
          unlisten = nextUnlisten;
        } else {
          nextUnlisten();
        }
      })
      .catch((err) => setError(formatCommandError(err)));

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  async function handleClose() {
    await hideTranslateResult();
  }

  return (
    <div className="translate-result-window">
      <header
        className="translate-result-header"
        title="拖拽移动翻译浮窗"
        onMouseDown={() => void startDragTranslateResultWindow()}
      >
        <span className="translate-result-title">译文</span>
        <button
          className="translate-result-close"
          type="button"
          onClick={(event) => {
            event.stopPropagation();
            void handleClose();
          }}
          onMouseDown={(event) => event.stopPropagation()}
          aria-label="关闭翻译浮窗"
        >
          ×
        </button>
      </header>
      <div className="translate-result-body">
        {error && <p role="alert">{error}</p>}
        {translatedText ? (
          <p className="translate-result-text" aria-label="译文内容">{translatedText}</p>
        ) : (
          <p className="translate-result-loading">正在翻译…</p>
        )}
      </div>
      <button
        className="translate-result-resize"
        type="button"
        aria-label="调整翻译浮窗大小"
        onMouseDown={(event) => {
          event.preventDefault();
          event.stopPropagation();
          void startResizeTranslateResultWindow('SouthEast');
        }}
      />
    </div>
  );
}
