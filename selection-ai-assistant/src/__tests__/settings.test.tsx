import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
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
    formatCommandError: (err: unknown) => {
      if (err instanceof Error) return err.message;
      if (err && typeof err === 'object' && 'message' in err) {
        const message = (err as { message?: unknown }).message;
        if (typeof message === 'string') return message;
      }
      return String(err);
    },
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

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
}

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

  it('shows Chinese provider form, current providers, clipboard warning, and disabled apps', async () => {
    render(<Settings />);

    expect(await screen.findByRole('heading', { name: '设置' })).toBeInTheDocument();
    expect(screen.getByLabelText('服务商 ID')).toHaveValue('openrouter');
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

    await waitFor(() => expect(screen.getByLabelText('模型')).toHaveValue('anthropic/claude-sonnet-4.5'));
    expect(screen.getByLabelText('服务商 ID')).toHaveValue('openrouter');
    expect(screen.getByLabelText('名称')).toHaveValue('OpenRouter');
  });

  it('saves provider config including API key and refreshes displayed provider list', async () => {
    const nextConfig = {
      ...config,
      providers: [{ ...config.providers[0], model: 'openai/gpt-4.1' }],
    };
    saveProviderConfigMock.mockResolvedValue(nextConfig);
    render(<Settings />);
    await screen.findByText(/anthropic\/claude-sonnet-4\.5/);

    const modelInput = screen.getByLabelText('模型');
    fireEvent.change(modelInput, { target: { value: '' } });
    fireEvent.change(modelInput, { target: { value: 'openai/gpt-4.1' } });
    const apiKeyInput = screen.getByLabelText('API 密钥');
    fireEvent.change(apiKeyInput, { target: { value: '' } });
    fireEvent.change(apiKeyInput, { target: { value: 'updated-dummy-key' } });
    fireEvent.click(screen.getByRole('button', { name: '保存服务商' }));

    expect(screen.getByRole('button', { name: '正在保存…' })).toBeDisabled();
    expect(screen.getByRole('status')).toHaveTextContent('正在保存服务商配置…');

    await waitFor(() => {
      expect(saveProviderConfigMock).toHaveBeenCalledWith(
        expect.objectContaining({ model: 'openai/gpt-4.1', apiKey: 'updated-dummy-key' }),
      );
    });
    expect(await screen.findByText(/OpenRouter — openai\/gpt-4\.1/)).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('已保存服务商配置。');
  });

  it('loads provider models with visible progress and selects the first model when no model is set', async () => {
    const modelsRequest = deferred<string[]>();
    listProviderModelsMock.mockReturnValueOnce(modelsRequest.promise);
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    const modelInput = screen.getByLabelText('模型');
    fireEvent.change(modelInput, { target: { value: '' } });
    fireEvent.click(screen.getByRole('button', { name: '加载模型列表' }));

    expect(screen.getByRole('button', { name: '正在加载模型…' })).toBeDisabled();
    expect(screen.getByRole('status')).toHaveTextContent('正在加载模型列表…');

    await act(async () => {
      modelsRequest.resolve(['openai/gpt-4.1', 'openai/gpt-4.1-mini']);
    });

    await waitFor(() => {
      expect(listProviderModelsMock).toHaveBeenCalledWith(expect.objectContaining({ apiKey: 'dummy-api-key' }));
    });
    expect(screen.getByRole('status')).toHaveTextContent('已加载 2 个模型。');
    expect(modelInput).toHaveValue('openai/gpt-4.1');

    const loadedModelSelect = screen.getByRole('combobox', { name: '已加载模型' });
    expect(within(loadedModelSelect).getByRole('option', { name: 'openai/gpt-4.1' })).toBeInTheDocument();
    expect(within(loadedModelSelect).getByRole('option', { name: 'openai/gpt-4.1-mini' })).toBeInTheDocument();

    fireEvent.change(loadedModelSelect, { target: { value: 'openai/gpt-4.1-mini' } });
    expect(modelInput).toHaveValue('openai/gpt-4.1-mini');
  });

  it('shows a visible error when loading provider models fails', async () => {
    listProviderModelsMock.mockRejectedValueOnce({ message: 'HTTP 401 Unauthorized' });
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    fireEvent.click(screen.getByRole('button', { name: '加载模型列表' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('加载模型失败：HTTP 401 Unauthorized');
    expect(screen.getByRole('button', { name: '加载模型列表' })).not.toBeDisabled();
  });

  it('tests provider connection with visible progress and shows model count', async () => {
    const connectionRequest = deferred<{ success: boolean; modelCount: number }>();
    testProviderConnectionMock.mockReturnValueOnce(connectionRequest.promise);
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    fireEvent.click(screen.getByRole('button', { name: '测试连接' }));

    expect(screen.getByRole('button', { name: '正在测试…' })).toBeDisabled();
    expect(screen.getByRole('status')).toHaveTextContent('正在测试服务商连接…');

    await act(async () => {
      connectionRequest.resolve({ success: true, modelCount: 2 });
    });

    await waitFor(() => {
      expect(testProviderConnectionMock).toHaveBeenCalledWith(expect.objectContaining({ baseUrl: 'https://openrouter.ai/api/v1' }));
    });
    expect(screen.getByRole('status')).toHaveTextContent('连接成功，可用模型 2 个。');
  });

  it('shows a visible error when provider connection test fails', async () => {
    testProviderConnectionMock.mockRejectedValueOnce({ message: 'network timeout' });
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    fireEvent.click(screen.getByRole('button', { name: '测试连接' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('测试连接失败：network timeout');
    expect(screen.getByRole('button', { name: '测试连接' })).not.toBeDisabled();
  });
});
