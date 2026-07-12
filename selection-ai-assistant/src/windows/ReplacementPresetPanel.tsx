import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import {
  focusFloatingButton,
  formatCommandError,
  getConfig,
  saveAppBehaviorConfig,
  setReplacementPresetPanelExpanded,
  type AppBehaviorConfig,
  type OutputTargetPreset,
  type TargetPresetKind,
} from '../api/tauri';

type TargetOption = {
  value: OutputTargetPreset;
  label: string;
  title: string;
};

type TargetPresetContext = {
  kind: TargetPresetKind;
};

const DEFAULT_APP_BEHAVIOR: AppBehaviorConfig = {
  hotkey: 'Ctrl+Alt+A',
  launchAtStartup: false,
  startMinimizedToTray: false,
  closeButtonBehavior: 'ask',
  replacementTargetLanguage: 'auto',
  replacementCustomTarget: '',
  translationTargetLanguage: 'auto',
  translationCustomTarget: '',
};

const TARGET_OPTIONS: TargetOption[] = [
  { value: 'auto', label: '自动', title: '中英自动互译' },
  { value: 'chinese', label: '中文', title: '输出中文' },
  { value: 'english', label: '英文', title: '输出英文' },
  { value: 'japanese', label: '日文', title: '输出日文' },
  { value: 'korean', label: '韩文', title: '输出韩文' },
  { value: 'classicalChinese', label: '文言', title: '改写成文言文' },
  { value: 'oracleBone', label: '甲骨', title: '甲骨文风格近似转写' },
  { value: 'pictograph', label: '象形', title: '象形文字风格近似转写' },
  { value: 'morseCode', label: '摩斯', title: '转换成摩斯密码' },
  { value: 'custom', label: '自定', title: '自定义输出或转换目标' },
];

function appBehaviorFromConfig(config: AppBehaviorConfig): AppBehaviorConfig {
  return {
    hotkey: config.hotkey,
    launchAtStartup: config.launchAtStartup,
    startMinimizedToTray: config.startMinimizedToTray,
    closeButtonBehavior: config.closeButtonBehavior,
    replacementTargetLanguage: config.replacementTargetLanguage,
    replacementCustomTarget: config.replacementCustomTarget,
    translationTargetLanguage: config.translationTargetLanguage,
    translationCustomTarget: config.translationCustomTarget,
  };
}

function selectedTarget(behavior: AppBehaviorConfig, kind: TargetPresetKind): OutputTargetPreset {
  return kind === 'translation' ? behavior.translationTargetLanguage : behavior.replacementTargetLanguage;
}

function selectedCustomTarget(behavior: AppBehaviorConfig, kind: TargetPresetKind): string {
  return kind === 'translation' ? behavior.translationCustomTarget : behavior.replacementCustomTarget;
}

function targetLabel(behavior: AppBehaviorConfig, kind: TargetPresetKind): string {
  const target = selectedTarget(behavior, kind);
  if (target === 'custom') {
    return selectedCustomTarget(behavior, kind).trim() || '自定义';
  }

  return TARGET_OPTIONS.find((option) => option.value === target)?.label ?? '自动';
}

function panelCopy(kind: TargetPresetKind) {
  if (kind === 'translation') {
    return {
      aria: '翻译输出目标设置',
      current: '翻译为',
      options: '选择翻译输出目标',
      emptyCustom: '输入自定义输出目标后回车保存',
      placeholder: '甲骨文风格 / 象形文字 / 摩斯密码',
    };
  }

  return {
    aria: '替换输出目标设置',
    current: '替换为',
    options: '选择替换输出目标',
    emptyCustom: '输入自定义输出目标后回车保存',
    placeholder: '韩语敬语 / 日文口语 / 文言文',
  };
}

export function ReplacementPresetPanel() {
  const [kind, setKind] = useState<TargetPresetKind>('replacement');
  const [appBehavior, setAppBehavior] = useState<AppBehaviorConfig>(DEFAULT_APP_BEHAVIOR);
  const [customTargetDraft, setCustomTargetDraft] = useState('');
  const [isCustomEditing, setIsCustomEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const currentLabel = targetLabel(appBehavior, kind);
  const currentTarget = selectedTarget(appBehavior, kind);
  const copy = panelCopy(kind);

  useEffect(() => {
    let active = true;

    getConfig()
      .then((config) => {
        if (!active) return;
        const nextBehavior = appBehaviorFromConfig(config);
        setAppBehavior(nextBehavior);
        setCustomTargetDraft(selectedCustomTarget(nextBehavior, kind));
      })
      .catch((err) => {
        if (active) setError(formatCommandError(err));
      });

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    listen<TargetPresetContext>('target_preset_context', (event) => {
      if (!active) return;
      setKind(event.payload.kind);
      setIsCustomEditing(false);
      setError(null);
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

  useEffect(() => {
    setCustomTargetDraft(selectedCustomTarget(appBehavior, kind));
    setIsCustomEditing(selectedTarget(appBehavior, kind) === 'custom');
  }, [
    appBehavior.replacementCustomTarget,
    appBehavior.replacementTargetLanguage,
    appBehavior.translationCustomTarget,
    appBehavior.translationTargetLanguage,
    kind,
  ]);

  useEffect(() => {
    void setReplacementPresetPanelExpanded(isCustomEditing || saving || Boolean(error)).catch((err: unknown) =>
      console.error('调整输出目标面板大小失败:', formatCommandError(err)),
    );
  }, [error, isCustomEditing, saving]);

  async function persistTargetConfig(targetLanguage: OutputTargetPreset, customTarget = customTargetDraft) {
    const normalizedCustomTarget = customTarget.trim();
    if (targetLanguage === 'custom' && !normalizedCustomTarget) {
      setAppBehavior({
        ...appBehavior,
        ...(kind === 'translation'
          ? { translationTargetLanguage: 'custom' as const }
          : { replacementTargetLanguage: 'custom' as const }),
      });
      setIsCustomEditing(true);
      setError(copy.emptyCustom);
      return;
    }

    const preferences: AppBehaviorConfig = {
      ...appBehavior,
      ...(kind === 'translation'
        ? {
            translationTargetLanguage: targetLanguage,
            translationCustomTarget:
              targetLanguage === 'custom' ? normalizedCustomTarget : appBehavior.translationCustomTarget,
          }
        : {
            replacementTargetLanguage: targetLanguage,
            replacementCustomTarget:
              targetLanguage === 'custom' ? normalizedCustomTarget : appBehavior.replacementCustomTarget,
          }),
    };

    setSaving(true);
    setError(null);
    setAppBehavior(preferences);

    try {
      const next = await saveAppBehaviorConfig(preferences);
      const nextBehavior = appBehaviorFromConfig(next);
      setAppBehavior(nextBehavior);
      setCustomTargetDraft(selectedCustomTarget(nextBehavior, kind));
      try {
        await focusFloatingButton();
      } catch (focusError) {
        console.error('恢复迷你操作条焦点失败:', formatCommandError(focusError));
      }
    } catch (err) {
      setError(formatCommandError(err));
    } finally {
      setSaving(false);
    }
  }

  async function handleTargetOptionClick(option: TargetOption) {
    if (option.value === 'custom') {
      setIsCustomEditing(true);
      if (!customTargetDraft.trim()) {
        setAppBehavior({
          ...appBehavior,
          ...(kind === 'translation'
            ? { translationTargetLanguage: 'custom' as const }
            : { replacementTargetLanguage: 'custom' as const }),
        });
        setError(null);
        return;
      }
    } else {
      setIsCustomEditing(false);
    }

    await persistTargetConfig(option.value);
  }

  return (
    <main className="replacement-preset-window">
      <section className="replacement-preset-panel" aria-label={copy.aria}>
        <div className="replacement-preset-strip">
          <span className="replacement-preset-current" title={`${copy.current}：${currentLabel}`}>
            {copy.current} <strong>{currentLabel}</strong>
          </span>
          <div className="replacement-language-options" role="group" aria-label={copy.options}>
            {TARGET_OPTIONS.map((option) => (
              <button
                key={option.value}
                className="replacement-language-option"
                type="button"
                title={option.title}
                aria-pressed={currentTarget === option.value}
                onClick={() => void handleTargetOptionClick(option)}
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
              placeholder={copy.placeholder}
              autoFocus
              onChange={(event) => setCustomTargetDraft(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault();
                  void persistTargetConfig('custom');
                }
              }}
            />
            <button type="button" disabled={saving} onClick={() => void persistTargetConfig('custom')}>
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
