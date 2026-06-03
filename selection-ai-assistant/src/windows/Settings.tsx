import { listen } from '@tauri-apps/api/event';
import { FormEvent, useEffect, useState } from 'react';
import {
  confirmMainWindowClose,
  formatCommandError,
  getConfig,
  getPlatformCapabilities,
  saveAppBehaviorConfig,
  saveProviderConfig,
  type AiProviderConfig,
  type AppBehaviorConfig,
  type AppConfig,
  type CloseButtonBehavior,
  type PlatformCapabilities,
} from '../api/tauri';
import { ProviderForm } from '../components/ProviderForm';

type Feedback = {
  kind: 'status' | 'error';
  message: string;
};

const defaultAppBehavior: AppBehaviorConfig = {
  startMinimizedToTray: false,
  closeButtonBehavior: 'ask',
};

function platformLabel(platform: PlatformCapabilities['platform']) {
  switch (platform) {
    case 'windows':
      return 'Windows';
    case 'macos':
      return 'macOS';
    case 'linux':
      return 'Linux';
    default:
      return '未知平台';
  }
}

function capabilityStatusLabel(status: PlatformCapabilities['automaticSelection']) {
  switch (status) {
    case 'supported':
      return '已支持';
    case 'permissionRequired':
      return '需要系统权限/平台实现';
    case 'unsupported':
      return '暂不支持';
    case 'unavailable':
      return '暂不可用';
  }
}

function PlatformCapabilitySummary({ capabilities }: { capabilities: PlatformCapabilities }) {
  const automaticSelectionLimited = capabilities.automaticSelection !== 'supported';

  return (
    <>
      <p>当前平台：{platformLabel(capabilities.platform)}</p>
      <dl className="platform-capability-list">
        <div>
          <dt>自动划词</dt>
          <dd>{capabilityStatusLabel(capabilities.automaticSelection)}</dd>
        </div>
        <div>
          <dt>输入监听</dt>
          <dd>{capabilityStatusLabel(capabilities.globalInputMonitor)}</dd>
        </div>
        <div>
          <dt>选区读取</dt>
          <dd>{capabilityStatusLabel(capabilities.selectionReader)}</dd>
        </div>
      </dl>
      {automaticSelectionLimited && (
        <p className="platform-capability-warning">
          当前平台暂未支持自动划词；可使用快捷键或手动输入文本复用 AI 面板，macOS/Linux 贡献者主要需要补系统层 backend。
        </p>
      )}
      {capabilities.permissionNote && <p className="field-help">{capabilities.permissionNote}</p>}
    </>
  );
}

export function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [platformCapabilities, setPlatformCapabilities] = useState<PlatformCapabilities | null>(null);
  const [appBehavior, setAppBehavior] = useState<AppBehaviorConfig>(defaultAppBehavior);
  const [behaviorSaving, setBehaviorSaving] = useState(false);
  const [behaviorFeedback, setBehaviorFeedback] = useState<Feedback | null>(null);
  const [closePromptOpen, setClosePromptOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const selectedProvider =
    config?.providers.find((provider) => provider.id === config.defaultProviderId) ?? config?.providers[0];

  useEffect(() => {
    getConfig()
      .then((loadedConfig) => {
        setConfig(loadedConfig);
        setAppBehavior({
          startMinimizedToTray: loadedConfig.startMinimizedToTray,
          closeButtonBehavior: loadedConfig.closeButtonBehavior,
        });
      })
      .catch((err) => setError(formatCommandError(err)));
  }, []);

  useEffect(() => {
    getPlatformCapabilities()
      .then(setPlatformCapabilities)
      .catch((err) => setError(formatCommandError(err)));
  }, []);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    listen('main_close_confirmation_requested', () => {
      setClosePromptOpen(true);
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

  async function handleSave(provider: AiProviderConfig) {
    const next = await saveProviderConfig(provider);
    setConfig(next);
  }

  async function handleSaveAppBehavior(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBehaviorSaving(true);
    setBehaviorFeedback({ kind: 'status', message: '正在保存启动与关闭设置…' });

    try {
      const next = await saveAppBehaviorConfig(appBehavior);
      setConfig(next);
      setAppBehavior({
        startMinimizedToTray: next.startMinimizedToTray,
        closeButtonBehavior: next.closeButtonBehavior,
      });
      setBehaviorFeedback({ kind: 'status', message: '已保存启动与关闭设置。' });
    } catch (err) {
      setBehaviorFeedback({ kind: 'error', message: `保存启动与关闭设置失败：${formatCommandError(err)}` });
    } finally {
      setBehaviorSaving(false);
    }
  }

  async function handleClosePromptChoice(behavior: CloseButtonBehavior) {
    setClosePromptOpen(false);

    try {
      const next = await confirmMainWindowClose(behavior);
      setConfig(next);
      setAppBehavior({
        startMinimizedToTray: next.startMinimizedToTray,
        closeButtonBehavior: next.closeButtonBehavior,
      });
    } catch (err) {
      setError(formatCommandError(err));
    }
  }

  return (
    <main className="settings-window">
      <header className="settings-hero">
        <p className="settings-kicker">Selection AI Assistant</p>
        <h1>设置</h1>
        <p>配置 OpenAI-compatible、Claude、Gemini 等多服务商协议、模型和本机隐私策略。</p>
      </header>

      {error && <p role="alert">{error}</p>}

      <div className="settings-grid">
        <section className="settings-section settings-section--primary">
          <h2>模型服务配置</h2>
          <ProviderForm initialProvider={selectedProvider} onSave={handleSave} />

          <section className="settings-section settings-section--embedded">
            <h2>启动与关闭</h2>
            <form className="app-behavior-form" onSubmit={handleSaveAppBehavior} aria-busy={behaviorSaving}>
              <div className="checkbox-field">
                <input
                  id="start-minimized-to-tray"
                  type="checkbox"
                  checked={appBehavior.startMinimizedToTray}
                  disabled={behaviorSaving}
                  onChange={(event) =>
                    setAppBehavior({ ...appBehavior, startMinimizedToTray: event.target.checked })
                  }
                />
                <label htmlFor="start-minimized-to-tray">启动时最小化到后台</label>
              </div>
              <p className="field-help">开启后启动应用时不显示设置窗口，只保留托盘后台运行。</p>

              <label>
                关闭按钮行为
                <select
                  value={appBehavior.closeButtonBehavior}
                  disabled={behaviorSaving}
                  onChange={(event) =>
                    setAppBehavior({ ...appBehavior, closeButtonBehavior: event.target.value as CloseButtonBehavior })
                  }
                >
                  <option value="ask">首次关闭时询问并记住选择</option>
                  <option value="minimizeToTray">最小化到后台</option>
                  <option value="exitApp">直接退出应用</option>
                </select>
              </label>

              <button type="submit" disabled={behaviorSaving}>
                {behaviorSaving ? '正在保存设置…' : '保存启动与关闭设置'}
              </button>
              {behaviorFeedback?.kind === 'status' && <p role="status">{behaviorFeedback.message}</p>}
              {behaviorFeedback?.kind === 'error' && <p role="alert">{behaviorFeedback.message}</p>}
            </form>
          </section>
        </section>

        <aside className="settings-side-panel" aria-label="当前配置概览">
          <section className="settings-section">
            <h2>平台支持</h2>
            {platformCapabilities ? <PlatformCapabilitySummary capabilities={platformCapabilities} /> : <p>正在读取平台能力…</p>}
          </section>

          <section className="settings-section">
            <h2>当前服务商</h2>
            {config?.providers.length ? (
              <ul>
                {config.providers.map((provider) => (
                  <li key={provider.id}>
                    {provider.name} — {provider.model || '未设置模型'}
                  </li>
                ))}
              </ul>
            ) : (
              <p>尚未配置服务商。</p>
            )}
          </section>

          <section className="settings-section">
            <h2>隐私</h2>
            {config?.clipboardFallbackEnabled && (
              <p className="privacy-warning">
                剪贴板兜底会短暂模拟复制选中文本。即使应用会恢复原剪贴板，Windows 剪贴板历史或第三方剪贴板管理器仍可能短暂记录该内容。敏感应用默认禁用。
              </p>
            )}
          </section>

          <section className="settings-section">
            <h2>禁用应用</h2>
            <ul>
              {(config?.disabledApps ?? []).map((app) => (
                <li key={app}>{app}</li>
              ))}
            </ul>
          </section>
        </aside>
      </div>

      {closePromptOpen && (
        <div className="close-confirm-backdrop">
          <section
            className="close-confirm-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="close-confirm-title"
          >
            <h2 id="close-confirm-title">关闭 Selection AI Assistant？</h2>
            <p>可以最小化到后台继续响应划词，也可以直接退出应用。本次选择会写入设置，之后可在“启动与关闭”中修改。</p>
            <div className="close-confirm-actions">
              <button type="button" onClick={() => void handleClosePromptChoice('minimizeToTray')}>
                最小化到后台并记住
              </button>
              <button type="button" onClick={() => void handleClosePromptChoice('exitApp')}>
                直接关闭并记住
              </button>
              <button type="button" onClick={() => setClosePromptOpen(false)}>
                取消
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}
