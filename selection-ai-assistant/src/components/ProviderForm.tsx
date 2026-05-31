import { FormEvent, useEffect, useState } from 'react';
import { formatCommandError, listProviderModels, testProviderConnection, type AiProviderConfig } from '../api/tauri';

type Props = {
  initialProvider?: AiProviderConfig;
  onSave: (provider: AiProviderConfig) => Promise<void>;
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
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    if (initialProvider) {
      setProvider(initialProvider);
    }
  }, [initialProvider]);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setSaving(true);
    setError(null);

    try {
      await onSave(provider);
    } catch (err) {
      setError(formatCommandError(err));
    } finally {
      setSaving(false);
    }
  }

  async function loadModels() {
    setLoadingModels(true);
    setError(null);
    setStatus(null);
    try {
      const loadedModels = await listProviderModels(provider);
      setModels(loadedModels);
      setStatus(`Loaded ${loadedModels.length} models.`);
      if (!provider.model && loadedModels[0]) {
        setProvider({ ...provider, model: loadedModels[0] });
      }
    } catch (err) {
      setError(formatCommandError(err));
    } finally {
      setLoadingModels(false);
    }
  }

  async function testConnection() {
    setTesting(true);
    setError(null);
    setStatus(null);
    try {
      const result = await testProviderConnection(provider);
      setStatus(`Connection successful. ${result.modelCount} models available.`);
    } catch (err) {
      setError(formatCommandError(err));
    } finally {
      setTesting(false);
    }
  }

  return (
    <form className="provider-form" onSubmit={submit}>
      <label>
        Provider ID
        <input value={provider.id} onChange={(event) => setProvider({ ...provider, id: event.target.value })} />
      </label>
      <label>
        Name
        <input value={provider.name} onChange={(event) => setProvider({ ...provider, name: event.target.value })} />
      </label>
      <label>
        Base URL
        <input value={provider.baseUrl} onChange={(event) => setProvider({ ...provider, baseUrl: event.target.value })} />
      </label>
      <label>
        Model
        <input
          list="provider-models"
          value={provider.model}
          onChange={(event) => setProvider({ ...provider, model: event.target.value })}
        />
      </label>
      <datalist id="provider-models">
        {models.map((model) => (
          <option key={model} value={model} />
        ))}
      </datalist>
      <button type="button" onClick={loadModels} disabled={loadingModels}>
        {loadingModels ? 'Loading models...' : 'Load Models'}
      </button>
      <button type="button" onClick={testConnection} disabled={testing}>
        {testing ? 'Testing...' : 'Test Connection'}
      </button>
      <label>
        API key
        <input
          type="password"
          value={provider.apiKey}
          onChange={(event) => setProvider({ ...provider, apiKey: event.target.value })}
          aria-describedby="api-key-help"
        />
      </label>
      <p id="api-key-help" className="field-help">
        Stored in your local settings file as plaintext. Environment variable fallback remains supported.
      </p>
      <label>
        API key secret reference (future storage)
        <input
          value={provider.apiKeyRef}
          onChange={(event) => setProvider({ ...provider, apiKeyRef: event.target.value })}
          aria-describedby="api-key-ref-help"
        />
      </label>
      <p id="api-key-ref-help" className="field-help">
        Optional label kept for future secure storage integration.
      </p>
      <button type="submit" disabled={saving}>
        {saving ? 'Saving...' : 'Save provider'}
      </button>
      {status && <p role="status">{status}</p>}
      {error && <p role="alert">{error}</p>}
    </form>
  );
}
