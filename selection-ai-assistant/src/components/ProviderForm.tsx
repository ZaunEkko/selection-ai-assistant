import { FormEvent, useState } from 'react';
import { formatCommandError, type AiProviderConfig } from '../api/tauri';

type Props = {
  onSave: (provider: AiProviderConfig) => Promise<void>;
};

export function ProviderForm({ onSave }: Props) {
  const [provider, setProvider] = useState<AiProviderConfig>({
    id: 'openrouter',
    name: 'OpenRouter',
    baseUrl: 'https://openrouter.ai/api/v1',
    model: '',
    apiKeyRef: 'credential://openrouter',
    headers: [],
  });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
        <input value={provider.model} onChange={(event) => setProvider({ ...provider, model: event.target.value })} />
      </label>
      <label>
        API key secret reference (future storage)
        <input
          value={provider.apiKeyRef}
          onChange={(event) => setProvider({ ...provider, apiKeyRef: event.target.value })}
          aria-describedby="api-key-ref-help"
        />
      </label>
      <p id="api-key-ref-help" className="field-help">
        This stores only a secret reference for future secure storage. Runtime AI requests currently read SELECTION_AI_API_KEY from the environment.
      </p>
      <button type="submit" disabled={saving}>
        {saving ? 'Saving...' : 'Save provider'}
      </button>
      {error && <p role="alert">{error}</p>}
    </form>
  );
}
