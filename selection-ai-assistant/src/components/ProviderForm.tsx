import { FormEvent, useEffect, useState } from 'react';
import {
  formatCommandError,
  listProviderModels,
  testProviderConnection,
  type AiProviderKind,
  type ProviderConfigView,
  type ProviderUpdate,
} from '../api/tauri';

type Props = {
  initialProvider?: ProviderConfigView;
  onSave: (provider: ProviderUpdate) => Promise<void>;
};

type ProviderDraft = {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  providerKind: AiProviderKind;
  apiKeyRef: string;
};

type Feedback = {
  kind: 'status' | 'error';
  message: string;
};

type ProviderPreset = {
  key: string;
  label: string;
  provider: ProviderDraft;
};

const providerPresets: ProviderPreset[] = [
  {
    key: 'openai',
    label: 'OpenAI',
    provider: {
      id: 'openai',
      name: 'OpenAI',
      baseUrl: 'https://api.openai.com/v1',
      model: 'gpt-4.1-mini',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://openai',
    },
  },
  {
    key: 'anthropic',
    label: 'Claude',
    provider: {
      id: 'anthropic',
      name: 'Claude',
      baseUrl: 'https://api.anthropic.com/v1',
      model: 'claude-sonnet-4-6',
      providerKind: 'anthropic',
      apiKeyRef: 'credential://anthropic',
    },
  },
  {
    key: 'gemini',
    label: 'Gemini',
    provider: {
      id: 'gemini',
      name: 'Gemini',
      baseUrl: 'https://generativelanguage.googleapis.com/v1beta',
      model: 'gemini-3.5-flash',
      providerKind: 'gemini',
      apiKeyRef: 'credential://gemini',
    },
  },
  {
    key: 'zhipu',
    label: '智谱 Zhipu',
    provider: {
      id: 'zhipu',
      name: '智谱 Zhipu',
      baseUrl: 'https://open.bigmodel.cn/api/paas/v4',
      model: 'glm-4.5',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://zhipu',
    },
  },
  {
    key: 'deepseek',
    label: 'DeepSeek',
    provider: {
      id: 'deepseek',
      name: 'DeepSeek',
      baseUrl: 'https://api.deepseek.com/v1',
      model: 'deepseek-chat',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://deepseek',
    },
  },
  {
    key: 'bailian',
    label: '阿里百炼 Bailian',
    provider: {
      id: 'bailian',
      name: '阿里百炼 Bailian',
      baseUrl: 'https://coding.dashscope.aliyuncs.com/v1',
      model: 'qwen-plus',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://bailian',
    },
  },
  {
    key: 'kimi',
    label: 'Kimi',
    provider: {
      id: 'kimi',
      name: 'Kimi',
      baseUrl: 'https://api.moonshot.cn/v1',
      model: 'moonshot-v1-8k',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://kimi',
    },
  },
  {
    key: 'minimax',
    label: 'Minimax',
    provider: {
      id: 'minimax',
      name: 'Minimax',
      baseUrl: 'https://api.minimax.io/v1',
      model: 'MiniMax-M1',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://minimax',
    },
  },
  {
    key: 'siliconflow',
    label: 'SiliconFlow',
    provider: {
      id: 'siliconflow',
      name: 'SiliconFlow',
      baseUrl: 'https://api.siliconflow.cn/v1',
      model: 'deepseek-ai/DeepSeek-V3',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://siliconflow',
    },
  },
  {
    key: 'aws-bedrock',
    label: 'AWS Bedrock',
    provider: {
      id: 'aws-bedrock',
      name: 'AWS Bedrock',
      baseUrl: 'https://bedrock-mantle.us-east-1.api.aws/v1',
      model: 'us.anthropic.claude-sonnet-4-5-20250929-v1:0',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://aws-bedrock',
    },
  },
  {
    key: 'volcengine',
    label: '火山方舟',
    provider: {
      id: 'volcengine',
      name: '火山方舟',
      baseUrl: 'https://ark.cn-beijing.volces.com/api/v3',
      model: 'doubao-seed-1-6',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://volcengine',
    },
  },
  {
    key: 'agentplan',
    label: 'AgentPlan',
    provider: {
      id: 'agentplan',
      name: 'AgentPlan',
      baseUrl: '',
      model: '',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://agentplan',
    },
  },
  {
    key: 'opencode',
    label: 'OpenCode',
    provider: {
      id: 'opencode',
      name: 'OpenCode',
      baseUrl: '',
      model: '',
      providerKind: 'openAiCompatible',
      apiKeyRef: 'credential://opencode',
    },
  },
];

const defaultProvider: ProviderDraft = {
  id: 'openrouter',
  name: 'OpenRouter',
  baseUrl: 'https://openrouter.ai/api/v1',
  model: '',
  providerKind: 'openAiCompatible',
  apiKeyRef: 'credential://openrouter',
};

function providerDraftFromView(provider: ProviderConfigView): ProviderDraft {
  return {
    id: provider.id,
    name: provider.name,
    baseUrl: provider.baseUrl,
    model: provider.model,
    providerKind: provider.providerKind,
    apiKeyRef: provider.apiKeyRef,
  };
}

export function ProviderForm({ initialProvider, onSave }: Props) {
  const [provider, setProvider] = useState<ProviderDraft>(
    initialProvider ? providerDraftFromView(initialProvider) : defaultProvider,
  );
  const [originalProviderId, setOriginalProviderId] = useState<string | null>(initialProvider?.id ?? null);
  const [apiKeyInput, setApiKeyInput] = useState('');
  const [clearApiKey, setClearApiKey] = useState(false);
  const [apiKeyConfigured, setApiKeyConfigured] = useState(initialProvider?.apiKeyConfigured ?? false);
  const [customHeadersConfigured, setCustomHeadersConfigured] = useState(
    initialProvider?.customHeadersConfigured ?? false,
  );
  const [saving, setSaving] = useState(false);
  const [loadingModels, setLoadingModels] = useState(false);
  const [testing, setTesting] = useState(false);
  const [models, setModels] = useState<string[]>([]);
  const [modelsOpen, setModelsOpen] = useState(false);
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const busy = saving || loadingModels || testing;

  useEffect(() => {
    const nextProvider = initialProvider ? providerDraftFromView(initialProvider) : defaultProvider;
    setProvider(nextProvider);
    setOriginalProviderId(initialProvider?.id ?? null);
    setApiKeyInput('');
    setClearApiKey(false);
    setApiKeyConfigured(initialProvider?.apiKeyConfigured ?? false);
    setCustomHeadersConfigured(initialProvider?.customHeadersConfigured ?? false);
  }, [initialProvider]);

  function currentProviderUpdate(): ProviderUpdate {
    return {
      originalProviderId,
      ...provider,
      apiKey:
        apiKeyInput.length > 0
          ? { action: 'replace', value: apiKeyInput }
          : clearApiKey
            ? { action: 'clear' }
            : { action: 'keep' },
    };
  }

  function applyPreset(key: string) {
    const preset = providerPresets.find((item) => item.key === key);
    if (!preset) return;

    const matchingInitialProvider = initialProvider?.id === preset.provider.id ? initialProvider : undefined;
    setProvider(preset.provider);
    setOriginalProviderId(matchingInitialProvider?.id ?? null);
    setApiKeyInput('');
    setClearApiKey(false);
    setApiKeyConfigured(matchingInitialProvider?.apiKeyConfigured ?? false);
    setCustomHeadersConfigured(matchingInitialProvider?.customHeadersConfigured ?? false);
    setModels([]);
    setModelsOpen(false);
    setFeedback(null);
  }

  function updateProviderKind(providerKind: AiProviderKind) {
    setProvider({ ...provider, providerKind });
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    setSaving(true);
    setFeedback({ kind: 'status', message: '正在保存服务商配置…' });

    try {
      await onSave(currentProviderUpdate());
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
      const loadedModels = await listProviderModels(currentProviderUpdate());
      setModels(loadedModels);
      setModelsOpen(false);
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
      const result = await testProviderConnection(currentProviderUpdate());
      setFeedback({
        kind: 'status',
        message: result.modelListAvailable
          ? `连接成功，可用模型 ${result.modelCount} 个。`
          : '连接成功，模型列表不可用，已使用当前模型完成兼容性测试。',
      });
    } catch (err) {
      setFeedback({ kind: 'error', message: `测试连接失败：${formatCommandError(err)}` });
    } finally {
      setTesting(false);
    }
  }

  function selectModel(model: string) {
    setProvider({ ...provider, model });
    setModelsOpen(false);
  }

  return (
    <form className="provider-form" onSubmit={submit} aria-busy={busy}>
      <label>
        厂商预设
        <select disabled={busy} defaultValue="" onChange={(event) => applyPreset(event.target.value)}>
          <option value="" disabled>
            选择厂商模板
          </option>
          {providerPresets.map((preset) => (
            <option key={preset.key} value={preset.key}>
              {preset.label}
            </option>
          ))}
        </select>
      </label>
      <label>
        协议类型
        <select
          disabled={busy}
          value={provider.providerKind}
          onChange={(event) => updateProviderKind(event.target.value as AiProviderKind)}
          aria-describedby="provider-kind-help"
        >
          <option value="openAiCompatible">OpenAI-compatible</option>
          <option value="anthropic">Claude / Anthropic Messages</option>
          <option value="gemini">Gemini Generative Language</option>
        </select>
      </label>
      <p id="provider-kind-help" className="field-help">
        OpenAI、DeepSeek、百炼、Kimi、Minimax、SiliconFlow、AWS Bedrock、火山等兼容接口使用 OpenAI-compatible；Claude 和 Gemini 使用官方原生协议。
      </p>
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
        填写当前协议的 base URL，例如 OpenAI 使用 https://api.openai.com/v1，Claude 使用 https://api.anthropic.com/v1，Gemini 使用 https://generativelanguage.googleapis.com/v1beta；不要包含具体模型路径。
      </p>
      <div className="model-field">
        <label htmlFor="provider-model-input">模型</label>
        <div className="model-combobox">
          <input
            id="provider-model-input"
            disabled={busy}
            value={provider.model}
            onChange={(event) => setProvider({ ...provider, model: event.target.value })}
            role={models.length > 0 ? 'combobox' : undefined}
            aria-expanded={models.length > 0 ? modelsOpen : undefined}
            aria-controls={models.length > 0 ? 'provider-model-options' : undefined}
            aria-autocomplete="list"
            aria-describedby="model-help"
          />
          {models.length > 0 && (
            <button
              type="button"
              className="model-combobox-toggle"
              disabled={busy}
              aria-label={modelsOpen ? '收起模型列表' : '展开模型列表'}
              onClick={() => setModelsOpen((open) => !open)}
            >
              ▾
            </button>
          )}
        </div>
        {models.length > 0 && modelsOpen && (
          <ul id="provider-model-options" className="model-options" role="listbox" aria-label="已加载模型">
            {models.map((model) => (
              <li
                key={model}
                role="option"
                aria-selected={provider.model === model}
                tabIndex={0}
                onClick={() => selectModel(model)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    selectModel(model);
                  }
                }}
              >
                {model}
              </li>
            ))}
          </ul>
        )}
      </div>
      <p id="model-help" className="field-help">
        可以手动输入模型名；加载模型后也可以从“已加载模型”下拉列表中选择。部分服务商不提供模型列表，手动填写模型名后仍可用当前协议测试连接。
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
          value={apiKeyInput}
          placeholder={apiKeyConfigured && !clearApiKey ? '留空以保留已保存密钥' : '输入新的 API 密钥'}
          onChange={(event) => {
            setApiKeyInput(event.target.value);
            setClearApiKey(false);
          }}
          aria-describedby="api-key-help"
        />
      </label>
      <p id="api-key-help" className="field-help">
        {clearApiKey
          ? '保存后会清除已保存的 API 密钥；SELECTION_AI_API_KEY 环境变量仍可作为兜底。'
          : apiKeyConfigured
            ? '已保存 API 密钥；留空会保持不变，输入新值会替换。密钥仍以明文保存在本机 settings 文件。'
            : 'API 密钥会以明文保存到本机 settings 文件；仍支持 SELECTION_AI_API_KEY 环境变量兜底。'}
      </p>
      {apiKeyConfigured && (
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            setApiKeyInput('');
            setClearApiKey((current) => !current);
          }}
        >
          {clearApiKey ? '撤销清除密钥' : '清除已保存密钥'}
        </button>
      )}
      {customHeadersConfigured && (
        <p className="field-help">已配置自定义请求头；保存时会保留现有值，但不会在界面或 IPC 中回显。</p>
      )}
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
