import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Settings } from '../windows/Settings';
import type { AppConfig } from '../api/tauri';

const { getConfigMock, saveProviderConfigMock } = vi.hoisted(() => ({
  getConfigMock: vi.fn(),
  saveProviderConfigMock: vi.fn(),
}));

vi.mock('../api/tauri', async () => {
  return {
    getConfig: getConfigMock,
    saveProviderConfig: saveProviderConfigMock,
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
    getConfigMock.mockResolvedValue(config);
    saveProviderConfigMock.mockResolvedValue(config);
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

  it('saves provider config and refreshes displayed provider list', async () => {
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
    fireEvent.click(screen.getByRole('button', { name: /save provider/i }));

    await waitFor(() => {
      expect(saveProviderConfigMock).toHaveBeenCalledWith(
        expect.objectContaining({ model: 'openai/gpt-4.1' }),
      );
    });
    expect(await screen.findByText(/OpenRouter — openai\/gpt-4\.1/)).toBeInTheDocument();
  });
});
