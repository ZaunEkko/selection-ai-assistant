import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Settings } from '../windows/Settings';
import type { AppConfig } from '../api/tauri';

const { getConfigMock, saveProviderConfigMock, listProviderModelsMock, testProviderConnectionMock } = vi.hoisted(() => ({
  getConfigMock: vi.fn(),
  saveProviderConfigMock: vi.fn(),
  listProviderModelsMock: vi.fn(),
  testProviderConnectionMock: vi.fn(),
}));

vi.mock('../api/tauri', async () => {
  return {
    getConfig: getConfigMock,
    saveProviderConfig: saveProviderConfigMock,
    listProviderModels: listProviderModelsMock,
    testProviderConnection: testProviderConnectionMock,
  };
});

const config: AppConfig = {
  defaultProviderId: 'openrouter',
  providers: [
    {
      id: 'openrouter',
      name: 'OpenRouter',
      baseUrl: 'https://openrouter.ai/api/v1',
      model: 'anthropic/claude-sonnet-4.5',
      apiKey: 'dummy-api-key',
      apiKeyRef: 'credential://openrouter',
      headers: [],
    },
  ],
  hoverRadius: 90,
  hoverDelayMs: 220,
  candidateTimeoutMs: 4000,
  minDragDistance: 6,
  hotkey: 'Ctrl+Alt+A',
  clipboardFallbackEnabled: true,
  showClipboardPrivacyWarningOnFirstUse: true,
  disableInElevatedWindows: true,
  manualHotkeyAlwaysEnabled: true,
  disabledApps: ['1Password.exe', 'KeePassXC.exe', 'Bitwarden.exe', 'mstsc.exe', 'AnyDesk.exe', 'TeamViewer.exe'],
};

describe('Settings', () => {
  beforeEach(() => {
    getConfigMock.mockReset();
    saveProviderConfigMock.mockReset();
    listProviderModelsMock.mockReset();
    testProviderConnectionMock.mockReset();
    getConfigMock.mockResolvedValue(config);
    saveProviderConfigMock.mockResolvedValue(config);
    listProviderModelsMock.mockResolvedValue(['openai/gpt-4.1', 'openai/gpt-4.1-mini']);
    testProviderConnectionMock.mockResolvedValue({ success: true, modelCount: 2 });
  });

  it('shows provider form, current providers, clipboard warning, and disabled apps', async () => {
    render(<Settings />);

    expect(await screen.findByRole('heading', { name: /settings/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/provider id/i)).toHaveValue('openrouter');
    expect(screen.getByText(/OpenRouter — anthropic\/claude-sonnet-4\.5/)).toBeInTheDocument();
    expect(screen.getByText(/剪贴板兜底会短暂模拟复制选中文本/)).toBeInTheDocument();

    for (const app of config.disabledApps) {
      expect(screen.getByText(app)).toBeInTheDocument();
    }
  });

  it('edits the default provider when it is not the first configured provider', async () => {
    getConfigMock.mockResolvedValue({
      ...config,
      defaultProviderId: 'openrouter',
      providers: [
        {
          id: 'openai',
          name: 'OpenAI',
          baseUrl: 'https://api.openai.com/v1',
          model: 'gpt-4.1-mini',
          apiKey: 'openai-key',
          apiKeyRef: 'credential://openai',
          headers: [],
        },
        {
          ...config.providers[0],
          model: 'anthropic/claude-sonnet-4.5',
        },
      ],
    });

    render(<Settings />);

    expect(await screen.findByLabelText(/provider id/i)).toHaveValue('openrouter');
    expect(screen.getByLabelText(/name/i)).toHaveValue('OpenRouter');
    expect(screen.getByLabelText(/model/i)).toHaveValue('anthropic/claude-sonnet-4.5');
  });

  it('saves provider config including API key and refreshes displayed provider list', async () => {
    const nextConfig = {
      ...config,
      providers: [{ ...config.providers[0], model: 'openai/gpt-4.1' }],
    };
    saveProviderConfigMock.mockResolvedValue(nextConfig);
    render(<Settings />);
    await screen.findByText(/anthropic\/claude-sonnet-4\.5/);

    const modelInput = screen.getByLabelText(/model/i);
    fireEvent.change(modelInput, { target: { value: '' } });
    fireEvent.change(modelInput, { target: { value: 'openai/gpt-4.1' } });
    const apiKeyInput = screen.getByLabelText(/api key$/i);
    fireEvent.change(apiKeyInput, { target: { value: '' } });
    fireEvent.change(apiKeyInput, { target: { value: 'updated-dummy-key' } });
    fireEvent.click(screen.getByRole('button', { name: /save provider/i }));

    await waitFor(() => {
      expect(saveProviderConfigMock).toHaveBeenCalledWith(
        expect.objectContaining({ model: 'openai/gpt-4.1', apiKey: 'updated-dummy-key' }),
      );
    });
    expect(await screen.findByText(/OpenRouter — openai\/gpt-4\.1/)).toBeInTheDocument();
  });

  it('loads provider models and selects the first model when no model is set', async () => {
    render(<Settings />);
    await screen.findByRole('heading', { name: /settings/i });

    const modelInput = screen.getByLabelText(/model/i);
    fireEvent.change(modelInput, { target: { value: '' } });
    fireEvent.click(screen.getByRole('button', { name: /load models/i }));

    await waitFor(() => {
      expect(listProviderModelsMock).toHaveBeenCalledWith(expect.objectContaining({ apiKey: 'dummy-api-key' }));
    });
    expect(await screen.findByRole('status')).toHaveTextContent('Loaded 2 models.');
    expect(modelInput).toHaveValue('openai/gpt-4.1');
  });

  it('tests provider connection and shows model count', async () => {
    render(<Settings />);
    await screen.findByRole('heading', { name: /settings/i });

    fireEvent.click(screen.getByRole('button', { name: /test connection/i }));

    await waitFor(() => {
      expect(testProviderConnectionMock).toHaveBeenCalledWith(expect.objectContaining({ baseUrl: 'https://openrouter.ai/api/v1' }));
    });
    expect(await screen.findByRole('status')).toHaveTextContent('Connection successful. 2 models available.');
  });
});
