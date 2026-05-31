import { beforeEach, describe, expect, it, vi } from 'vitest';
import { listProviderModels, openPanelFromFloatingButton, testProviderConnection, type AiProviderConfig } from '../api/tauri';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

describe('Tauri API wrappers', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('opens the AI panel from the stored current selection when the floating button is clicked', async () => {
    invokeMock.mockResolvedValue({});

    await openPanelFromFloatingButton();

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('open_panel_for_current_selection');
  });

  it('invokes provider model and connection commands with provider payloads', async () => {
    const provider: AiProviderConfig = {
      id: 'openai',
      name: 'OpenAI',
      baseUrl: 'https://api.openai.com/v1',
      model: '',
      apiKey: 'dummy-api-key',
      apiKeyRef: 'credential://openai',
      headers: [],
    };
    invokeMock.mockResolvedValueOnce(['gpt-test']).mockResolvedValueOnce({ success: true, modelCount: 1 });

    await expect(listProviderModels(provider)).resolves.toEqual(['gpt-test']);
    await expect(testProviderConnection(provider)).resolves.toEqual({ success: true, modelCount: 1 });

    expect(invokeMock).toHaveBeenNthCalledWith(1, 'list_provider_models', { provider });
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'test_provider_connection', { provider });
  });
});
