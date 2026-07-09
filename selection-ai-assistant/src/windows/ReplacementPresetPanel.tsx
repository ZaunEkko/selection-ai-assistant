import { useEffect, useState } from 'react';
import {
  formatCommandError,
  getConfig,
  saveAppBehaviorConfig,
  type AppBehaviorConfig,
  type ReplacementTargetLanguage,
} from '../api/tauri';

type ReplacementOption = {
  value: ReplacementTargetLanguage;
  label: string;
};

const DEFAULT_APP_BEHAVIOR: AppBehaviorConfig = {
  hotkey: 'Ctrl+Alt+A',
  launchAtStartup: false,
  startMinimizedToTray: false,
  closeButtonBehavior: 'ask',
  replacementTargetLanguage: 'auto',
  replacementCustomTarget: '',
};

const REPLACEMENT_OPTIONS: ReplacementOption[] = [
  { value: 'auto', label: '自动' },
  { value: 'chinese', label: '中文' },
  { value: 'english', label: '英文' },
  { value: 'japanese', label: '日文' },
  { value: 'korean', label: '韩文' },
  { value: 'custom', label: '自定' },
];

function appBehaviorFromConfig(config: AppBehaviorConfig): AppBehaviorConfig {
  return {
    hotkey: config.hotkey,
    launchAtStartup: config.launchAtStartup,
    startMinimizedToTray: config.startMinimizedToTray,
    closeButtonBehavior: config.closeButtonBehavior,
    replacementTargetLanguage: config.replacementTargetLanguage,
    replacementCustomTarget: config.replacementCustomTarget,
  };
}

function replacementTargetLabel(behavior: AppBehaviorConfig): string {
  if (behavior.replacementTargetLanguage === 'custom') {
    return behavior.replacementCustomTarget.trim() || '自定义';
  }

  return REPLACEMENT_OPTIONS.find((option) => option.value === behavior.replacementTargetLanguage)?.label ?? '自动';
}

export function ReplacementPresetPanel() {
  const [appBehavior, setAppBehavior] = useState<AppBehaviorConfig>(DEFAULT_APP_BEHAVIOR);
  const [customTargetDraft, setCustomTargetDraft] = useState('');
  const [isCustomEditing, setIsCustomEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const currentLabel = replacementTargetLabel(appBehavior);

  useEffect(() => {
    let active = true;

    getConfig()
      .then((config) => {
        if (!active) return;
        const nextBehavior = appBehaviorFromConfig(config);
        setAppBehavior(nextBehavior);
        setCustomTargetDraft(nextBehavior.replacementCustomTarget);
      })
      .catch((err) => {
        if (active) setError(formatCommandError(err));
      });

    return () => {
      active = false;
    };
  }, []);

  async function persistReplacementConfig(targetLanguage: ReplacementTargetLanguage, customTarget = customTargetDraft) {
    const normalizedCustomTarget = customTarget.trim();
    if (targetLanguage === 'custom' && !normalizedCustomTarget) {
      setAppBehavior({ ...appBehavior, replacementTargetLanguage: 'custom' });
      setIsCustomEditing(true);
      setError('输入自定义目标后回车保存');
      return;
    }

    const preferences: AppBehaviorConfig = {
      ...appBehavior,
      replacementTargetLanguage: targetLanguage,
      replacementCustomTarget: targetLanguage === 'custom' ? normalizedCustomTarget : customTargetDraft,
    };

    setSaving(true);
    setError(null);
    setAppBehavior(preferences);

    try {
      const next = await saveAppBehaviorConfig(preferences);
      const nextBehavior = appBehaviorFromConfig(next);
      setAppBehavior(nextBehavior);
      setCustomTargetDraft(nextBehavior.replacementCustomTarget);
    } catch (err) {
      setError(formatCommandError(err));
    } finally {
      setSaving(false);
    }
  }

  async function handleReplacementOptionClick(option: ReplacementOption) {
    if (option.value === 'custom') {
      setIsCustomEditing(true);
      if (!customTargetDraft.trim()) {
        setAppBehavior({ ...appBehavior, replacementTargetLanguage: 'custom' });
        setError(null);
        return;
      }
    } else {
      setIsCustomEditing(false);
    }

    await persistReplacementConfig(option.value);
  }

  return (
    <main className="replacement-preset-window">
      <section className="replacement-preset-panel" aria-label="替换目标设置">
        <div className="replacement-preset-strip">
          <span className="replacement-preset-current" title={`当前替换目标：${currentLabel}`}>
            替换为 <strong>{currentLabel}</strong>
          </span>
          <div className="replacement-language-options" role="group" aria-label="选择替换目标">
            {REPLACEMENT_OPTIONS.map((option) => (
              <button
                key={option.value}
                className="replacement-language-option"
                type="button"
                aria-pressed={appBehavior.replacementTargetLanguage === option.value}
                onClick={() => void handleReplacementOptionClick(option)}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>
        {isCustomEditing ? (
          <label className="replacement-custom-target">
            <input
              type="text"
              value={customTargetDraft}
              placeholder="韩语敬语 / 日文口语"
              autoFocus
              onChange={(event) => setCustomTargetDraft(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault();
                  void persistReplacementConfig('custom');
                }
              }}
            />
            <button type="button" disabled={saving} onClick={() => void persistReplacementConfig('custom')}>
              保存
            </button>
          </label>
        ) : null}
        {error || saving ? (
          <p className="replacement-preset-message" role={error ? 'alert' : 'status'}>
            {error ?? '保存中…'}
          </p>
        ) : null}
      </section>
    </main>
  );
}
