import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';
import { formatCommandError } from '../api/tauri';

type TranslateResultEvent = {
  originalText: string;
  translatedText: string;
};

export function TranslateResult() {
  const [originalText, setOriginalText] = useState<string>('');
  const [translatedText, setTranslatedText] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    listen<TranslateResultEvent>('translate_result', (event) => {
      if (active) {
        setOriginalText(event.payload.originalText);
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
    await getCurrentWindow().hide();
  }

  return (
    <div className="translate-result-window">
      <header className="translate-result-header">
        <span className="translate-result-title">翻译对照</span>
        <button className="translate-result-close" type="button" onClick={() => void handleClose()} aria-label="关闭">
          ×
        </button>
      </header>
      <div className="translate-result-body">
        {error && <p role="alert">{error}</p>}
        {translatedText ? (
          <div className="translate-result-compare" aria-label="翻译对照">
            <section>
              <h2>原文</h2>
              <p className="translate-result-source">{originalText}</p>
            </section>
            <section>
              <h2>译文</h2>
              <p className="translate-result-text">{translatedText}</p>
            </section>
          </div>
        ) : (
          <p className="translate-result-loading">正在翻译…</p>
        )}
      </div>
    </div>
  );
}
