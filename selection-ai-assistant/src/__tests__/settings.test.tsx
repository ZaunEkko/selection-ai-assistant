import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Settings } from '../windows/Settings';
import type { AppBehaviorConfig, SettingsConfigView } from '../api/tauri';

type Listener<T = unknown> = (event: { payload: T }) => void;

const {
  listeners,
  getConfigMock,
  saveProviderConfigMock,
  saveAppBehaviorConfigMock,
  confirmMainWindowCloseMock,
  getPlatformCapabilitiesMock,
  listProviderModelsMock,
  testProviderConnectionMock,
} = vi.hoisted(() => ({
  listeners: new Map<string, Listener[]>(),
  getConfigMock: vi.fn(),
  saveProviderConfigMock: vi.fn(),
  saveAppBehaviorConfigMock: vi.fn(),
  confirmMainWindowCloseMock: vi.fn(),
  getPlatformCapabilitiesMock: vi.fn(),
  listProviderModelsMock: vi.fn(),
  testProviderConnectionMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (eventName: string, callback: Listener) => {
    const existing = listeners.get(eventName) ?? [];
    existing.push(callback);
    listeners.set(eventName, existing);
    return Promise.resolve(() => {
      listeners.set(
        eventName,
        (listeners.get(eventName) ?? []).filter((item) => item !== callback),
      );
    });
  },
}));

vi.mock('../api/tauri', async () => {
  return {
    getConfig: getConfigMock,
    saveProviderConfig: saveProviderConfigMock,
    saveAppBehaviorConfig: saveAppBehaviorConfigMock,
    confirmMainWindowClose: confirmMainWindowCloseMock,
    getPlatformCapabilities: getPlatformCapabilitiesMock,
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

const config: SettingsConfigView = {
  defaultProviderId: 'openrouter',
  providers: [
    {
      id: 'openrouter',
      name: 'OpenRouter',
      baseUrl: 'https://openrouter.ai/api/v1',
      model: 'anthropic/claude-sonnet-4.5',
      providerKind: 'openAiCompatible',
      apiKeyConfigured: true,
      apiKeyRef: 'credential://openrouter',
      customHeadersConfigured: false,
    },
  ],
  hotkey: 'Ctrl+Alt+A',
  launchAtStartup: false,
  clipboardFallbackEnabled: true,
  startMinimizedToTray: false,
  closeButtonBehavior: 'ask',
  replacementTargetLanguage: 'auto',
  replacementCustomTarget: '',
  translationTargetLanguage: 'auto',
  translationCustomTarget: '',
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

function emit<T>(eventName: string, payload: T) {
  for (const listener of listeners.get(eventName) ?? []) {
    listener({ payload });
  }
}

describe('Settings', () => {
  beforeEach(() => {
    getConfigMock.mockReset();
    saveProviderConfigMock.mockReset();
    saveAppBehaviorConfigMock.mockReset();
    confirmMainWindowCloseMock.mockReset();
    getPlatformCapabilitiesMock.mockReset();
    listProviderModelsMock.mockReset();
    testProviderConnectionMock.mockReset();
    listeners.clear();
    getConfigMock.mockResolvedValue(config);
    saveProviderConfigMock.mockResolvedValue(config);
    saveAppBehaviorConfigMock.mockResolvedValue(config);
    confirmMainWindowCloseMock.mockResolvedValue(config);
    getPlatformCapabilitiesMock.mockResolvedValue({
      platform: 'windows',
      automaticSelection: 'supported',
      globalInputMonitor: 'supported',
      selectionReader: 'supported',
      selectionAnchorReader: 'supported',
      clipboardFallback: 'supported',
      manualHotkey: 'supported',
      permissionCheck: 'supported',
      permissionNote: null,
    });
    listProviderModelsMock.mockResolvedValue(['openai/gpt-4.1', 'openai/gpt-4.1-mini']);
    testProviderConnectionMock.mockResolvedValue({ success: true, modelCount: 2, modelListAvailable: true });
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

  it('does not return the saved API key to the provider form', async () => {
    render(<Settings />);

    await screen.findByText(/已保存 API 密钥；留空会保持不变/);
    const apiKeyInput = screen.getByLabelText('API 密钥');
    expect(apiKeyInput).toHaveValue('');
    expect(apiKeyInput).toHaveAttribute('placeholder', '留空以保留已保存密钥');
  });

  it('sends an explicit clear action for a saved API key', async () => {
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    fireEvent.click(await screen.findByRole('button', { name: '清除已保存密钥' }));
    expect(screen.getByText(/保存后会清除已保存的 API 密钥/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '保存服务商' }));

    await waitFor(() => {
      expect(saveProviderConfigMock).toHaveBeenCalledWith(
        expect.objectContaining({ originalProviderId: 'openrouter', apiKey: { action: 'clear' } }),
      );
    });
  });

  it('shows platform fallback guidance when automatic selection is not available', async () => {
    getPlatformCapabilitiesMock.mockResolvedValueOnce({
      platform: 'linux',
      automaticSelection: 'unavailable',
      globalInputMonitor: 'unsupported',
      selectionReader: 'unavailable',
      selectionAnchorReader: 'unavailable',
      clipboardFallback: 'unavailable',
      manualHotkey: 'unavailable',
      permissionCheck: 'unavailable',
      permissionNote: 'Linux backend 已预留；Wayland 默认限制更强。',
    });

    render(<Settings />);

    expect(await screen.findByText(/当前平台：Linux/)).toBeInTheDocument();
    expect(screen.getByText(/当前平台暂未支持自动划词/)).toBeInTheDocument();
    expect(screen.getByText(/快捷键或手动输入/)).toBeInTheDocument();
    expect(screen.getByText(/Wayland 默认限制更强/)).toBeInTheDocument();
  });

  it('saves startup, close button behavior, and screenshot hotkey preferences', async () => {
    const nextConfig: AppBehaviorConfig = {
      hotkey: 'Ctrl+Alt+T',
      launchAtStartup: true,
      startMinimizedToTray: true,
      closeButtonBehavior: 'exitApp',
      replacementTargetLanguage: 'auto',
      replacementCustomTarget: '',
      translationTargetLanguage: 'auto',
      translationCustomTarget: '',
    };
    saveAppBehaviorConfigMock.mockResolvedValue(nextConfig);
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    fireEvent.click(screen.getByLabelText('开机自启'));
    fireEvent.click(screen.getByLabelText('启动时最小化到后台'));
    fireEvent.change(screen.getByLabelText('关闭按钮行为'), { target: { value: 'exitApp' } });
    fireEvent.change(screen.getByLabelText('截图翻译快捷键'), { target: { value: 'Ctrl+Alt+T' } });
    fireEvent.click(screen.getByRole('button', { name: '保存启动、后台与截图快捷键设置' }));

    expect(screen.getByRole('button', { name: '正在保存设置…' })).toBeDisabled();
    expect(screen.getByRole('status')).toHaveTextContent('正在保存启动、后台与截图快捷键设置…');

    await waitFor(() => {
      expect(saveAppBehaviorConfigMock).toHaveBeenCalledWith({
        hotkey: 'Ctrl+Alt+T',
        launchAtStartup: true,
        startMinimizedToTray: true,
        closeButtonBehavior: 'exitApp',
        replacementTargetLanguage: 'auto',
        replacementCustomTarget: '',
        translationTargetLanguage: 'auto',
        translationCustomTarget: '',
      });
    });
    expect(screen.getByLabelText('截图翻译快捷键')).toHaveValue('Ctrl+Alt+T');
    expect(screen.getByLabelText('开机自启')).toBeChecked();
    expect(screen.getByLabelText('启动时最小化到后台')).toBeChecked();
    expect(screen.getByLabelText('关闭按钮行为')).toHaveValue('exitApp');
    expect(screen.getByRole('status')).toHaveTextContent('已保存启动、后台与截图快捷键设置。');
  });

  it('prompts on the first main window close request and remembers the selected close behavior', async () => {
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    await act(async () => {
      emit('main_close_confirmation_requested', null);
    });

    expect(screen.getByRole('dialog', { name: '关闭 Selection AI Assistant？' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '最小化到后台并记住' }));

    await waitFor(() => expect(confirmMainWindowCloseMock).toHaveBeenCalledWith('minimizeToTray'));
    expect(screen.queryByRole('dialog', { name: '关闭 Selection AI Assistant？' })).not.toBeInTheDocument();
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
          providerKind: 'openAiCompatible',
          apiKeyConfigured: true,
          apiKeyRef: 'credential://openai',
          customHeadersConfigured: false,
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

  it('applies official provider presets with protocol specific defaults', async () => {
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    fireEvent.change(screen.getByLabelText('厂商预设'), { target: { value: 'anthropic' } });

    expect(screen.getByLabelText('服务商 ID')).toHaveValue('anthropic');
    expect(screen.getByLabelText('名称')).toHaveValue('Claude');
    expect(screen.getByLabelText('接口地址')).toHaveValue('https://api.anthropic.com/v1');
    expect(screen.getByLabelText('协议类型')).toHaveValue('anthropic');
    expect(screen.getByLabelText('模型')).toHaveValue('claude-sonnet-4-6');
  });

  it('does not carry saved credentials into a different provider preset', async () => {
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    fireEvent.change(screen.getByLabelText('厂商预设'), { target: { value: 'anthropic' } });
    fireEvent.click(screen.getByRole('button', { name: '加载模型列表' }));

    await waitFor(() => {
      expect(listProviderModelsMock).toHaveBeenCalledWith(
        expect.objectContaining({
          originalProviderId: null,
          id: 'anthropic',
          apiKey: { action: 'keep' },
        }),
      );
    });
  });

  it('includes required domestic and platform provider presets', async () => {
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    const presetSelect = screen.getByLabelText('厂商预设');
    for (const name of [
      'OpenAI',
      'Claude',
      'Gemini',
      '智谱 Zhipu',
      'DeepSeek',
      '阿里百炼 Bailian',
      'Kimi',
      'Minimax',
      'SiliconFlow',
      'AWS Bedrock',
      '火山方舟',
      'AgentPlan',
      'OpenCode',
    ]) {
      expect(within(presetSelect).getByRole('option', { name })).toBeInTheDocument();
    }
  });

  it('saves provider config with an explicit API key replacement and refreshes the provider list', async () => {
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
        expect.objectContaining({
          originalProviderId: 'openrouter',
          model: 'openai/gpt-4.1',
          apiKey: { action: 'replace', value: 'updated-dummy-key' },
        }),
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
      expect(listProviderModelsMock).toHaveBeenCalledWith(
        expect.objectContaining({ originalProviderId: 'openrouter', apiKey: { action: 'keep' } }),
      );
    });
    expect(screen.getByRole('status')).toHaveTextContent('已加载 2 个模型。');
    expect(modelInput).toHaveValue('openai/gpt-4.1');

    expect(screen.queryByRole('combobox', { name: '已加载模型' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '展开模型列表' }));
    const modelList = screen.getByRole('listbox', { name: '已加载模型' });
    expect(within(modelList).getByRole('option', { name: 'openai/gpt-4.1' })).toBeInTheDocument();
    expect(within(modelList).getByRole('option', { name: 'openai/gpt-4.1-mini' })).toBeInTheDocument();

    fireEvent.click(within(modelList).getByRole('option', { name: 'openai/gpt-4.1-mini' }));
    expect(modelInput).toHaveValue('openai/gpt-4.1-mini');
    expect(screen.queryByRole('listbox', { name: '已加载模型' })).not.toBeInTheDocument();
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
    const connectionRequest = deferred<{ success: boolean; modelCount: number; modelListAvailable: boolean }>();
    testProviderConnectionMock.mockReturnValueOnce(connectionRequest.promise);
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    fireEvent.click(screen.getByRole('button', { name: '测试连接' }));

    expect(screen.getByRole('button', { name: '正在测试…' })).toBeDisabled();
    expect(screen.getByRole('status')).toHaveTextContent('正在测试服务商连接…');

    await act(async () => {
      connectionRequest.resolve({ success: true, modelCount: 2, modelListAvailable: true });
    });

    await waitFor(() => {
      expect(testProviderConnectionMock).toHaveBeenCalledWith(expect.objectContaining({ baseUrl: 'https://openrouter.ai/api/v1' }));
    });
    expect(screen.getByRole('status')).toHaveTextContent('连接成功，可用模型 2 个。');
  });

  it('shows chat probe success when the provider does not expose a model list endpoint', async () => {
    testProviderConnectionMock.mockResolvedValueOnce({ success: true, modelCount: 0, modelListAvailable: false });
    render(<Settings />);
    await screen.findByRole('heading', { name: '设置' });

    fireEvent.click(screen.getByRole('button', { name: '测试连接' }));

    await waitFor(() => {
      expect(testProviderConnectionMock).toHaveBeenCalledWith(expect.objectContaining({ model: 'anthropic/claude-sonnet-4.5' }));
    });
    expect(screen.getByRole('status')).toHaveTextContent('连接成功，模型列表不可用，已使用当前模型完成兼容性测试。');
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
