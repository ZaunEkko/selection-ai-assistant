import { FormEvent, useEffect, useState } from 'react';
import { formatCommandError, listProviderModels, testProviderConnection, type AiProviderConfig } from '../api/tauri';

type Props = {
  initialProvider?: AiProviderConfig;
  onSave: (provider: AiProviderConfig) => Promise<void>;
};

type Feedback = {
  kind: 'status' | 'error';
  message: string;
};

const defaultProvider: AiProviderConfig = {
  id: 'openrouter',
  name: 'OpenRouter',
  baseUrl: 'https://openrouter.ai/api/v1',
  model: '',
  apiKey: '',
  apiKeyRef: 'credential://openrouter',
  headers: [],
};

export function ProviderForm({ initialProvider, onSave }: Props) {
  const [provider, setProvider] = useState<AiProviderConfig>(initialProvider ?? defaultProvider);
  const [saving, setSaving] = useState(false);
  const [loadingModels, setLoadingModels] = useState(false);
  const [testing, setTesting] = useState(false);
  const [models, setModels] = useState<string[]>([]);
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const busy = saving || loadingModels || testing;
  const selectedLoadedModel = models.includes(provider.model) ? provider.model : '';

  useEffect(() => {
    if (initialProvider) {
      setProvider(initialProvider);
    }
  }, [initialProvider]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setSaving(true);
    setFeedback({ kind: 'status', message: '正在保存服务商配置…' });

    try {
      await onSave(provider);
      setFeedback({ kind: 'status', message: '已保存服务商配置。' });
    } catch (err) {
      setFeedback({ kind: 'error', message: `保存失败：${formatCommandError(err)}` });
    } finally {
      setSaving(false);
    }
  }

  async function loadModels() {
    setLoadingModels(true);
    setFeedback({ kind: 'status', message: '正在加载模型列表…' });
    try {
      const loadedModels = await listProviderModels(provider);
      setModels(loadedModels);
      setFeedback({ kind: 'status', message: `已加载 ${loadedModels.length} 个模型。` });
      if (loadedModels[0]) {
        setProvider((current) => (current.model ? current : { ...current, model: loadedModels[0] }));
      }
    } catch (err) {
      setFeedback({ kind: 'error', message: `加载模型失败：${formatCommandError(err)}` });
    } finally {
      setLoadingModels(false);
    }
  }

  async function testConnection() {
    setTesting(true);
    setFeedback({ kind: 'status', message: '正在测试服务商连接…' });
    try {
      const result = await testProviderConnection(provider);
      setFeedback({ kind: 'status', message: `连接成功，可用模型 ${result.modelCount} 个。` });
    } catch (err) {
      setFeedback({ kind: 'error', message: `测试连接失败：${formatCommandError(err)}` });
    } finally {
      setTesting(false);
    }
  }

  return (
    <form className="provider-form" onSubmit={submit} aria-busy={busy}>
      <label>
        服务商 ID
        <input disabled={busy} value={provider.id} onChange={(event) => setProvider({ ...provider, id: event.target.value })} />
      </label>
      <label>
        名称
        <input disabled={busy} value={provider.name} onChange={(event) => setProvider({ ...provider, name: event.target.value })} />
      </label>
      <label>
        接口地址
        <input
          disabled={busy}
          value={provider.baseUrl}
          onChange={(event) => setProvider({ ...provider, baseUrl: event.target.value })}
          aria-describedby="base-url-help"
        />
      </label>
      <p id="base-url-help" className="field-help">
        填写 OpenAI-compatible base URL，例如 https://api.openai.com/v1；不要包含 /models。
      </p>
      <div className="model-field">
        <label htmlFor="provider-model-input">模型</label>
        <input
          id="provider-model-input"
          disabled={busy}
          value={provider.model}
          onChange={(event) => setProvider({ ...provider, model: event.target.value })}
          aria-describedby="model-help"
        />
        {models.length > 0 && (
          <select
            aria-label="已加载模型"
            disabled={busy}
            value={selectedLoadedModel}
            onChange={(event) => setProvider({ ...provider, model: event.target.value })}
          >
            <option value="">选择已加载模型</option>
            {models.map((model) => (
              <option key={model} value={model}>
                {model}
              </option>
            ))}
          </select>
        )}
      </div>
      <p id="model-help" className="field-help">
        可以手动输入模型名；加载模型后也可以从“已加载模型”下拉列表中选择。
      </p>
      <div className="provider-actions">
        <button type="button" onClick={loadModels} disabled={busy}>
          {loadingModels ? '正在加载模型…' : '加载模型列表'}
        </button>
        <button type="button" onClick={testConnection} disabled={busy}>
          {testing ? '正在测试…' : '测试连接'}
        </button>
      </div>
      <label>
        API 密钥
        <input
          disabled={busy}
          type="password"
          value={provider.apiKey}
          onChange={(event) => setProvider({ ...provider, apiKey: event.target.value })}
          aria-describedby="api-key-help"
        />
      </label>
      <p id="api-key-help" className="field-help">
        API 密钥会以明文保存到本机 settings 文件；仍支持 SELECTION_AI_API_KEY 环境变量兜底。
      </p>
      <label>
        API 密钥引用（未来安全存储）
        <input
          disabled={busy}
          value={provider.apiKeyRef}
          onChange={(event) => setProvider({ ...provider, apiKeyRef: event.target.value })}
          aria-describedby="api-key-ref-help"
        />
      </label>
      <p id="api-key-ref-help" className="field-help">
        可选标签，预留给未来系统凭据存储集成。
      </p>
      <button type="submit" disabled={busy}>
        {saving ? '正在保存…' : '保存服务商'}
      </button>
      {feedback?.kind === 'status' && <p role="status">{feedback.message}</p>}
      {feedback?.kind === 'error' && <p role="alert">{feedback.message}</p>}
    </form>
  );
}
