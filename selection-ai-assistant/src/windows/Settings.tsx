import { useEffect, useState } from 'react';
import { formatCommandError, getConfig, saveProviderConfig, type AiProviderConfig, type AppConfig } from '../api/tauri';
import { ProviderForm } from '../components/ProviderForm';

export function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const selectedProvider =
    config?.providers.find((provider) => provider.id === config.defaultProviderId) ?? config?.providers[0];

  useEffect(() => {
    getConfig()
      .then(setConfig)
      .catch((err) => setError(formatCommandError(err)));
  }, []);

  async function handleSave(provider: AiProviderConfig) {
    const next = await saveProviderConfig(provider);
    setConfig(next);
  }

  return (
    <main className="settings-window">
      <h1>设置</h1>
      {error && <p role="alert">{error}</p>}

      <section className="settings-section">
        <h2>模型服务配置</h2>
        <ProviderForm initialProvider={selectedProvider} onSave={handleSave} />
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
    </main>
  );
}
