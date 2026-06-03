import { describe, expect, it } from 'vitest';
import { formatCommandError } from '../api/tauri';

describe('formatCommandError', () => {
  it('formats common Tauri public error objects in Chinese', () => {
    expect(formatCommandError({ code: 'api_key_missing', message: 'Set SELECTION_AI_API_KEY before running an AI action.' })).toBe(
      '请在设置中填写 API 密钥，或配置 SELECTION_AI_API_KEY 环境变量。',
    );
  });

  it('uses Error messages and falls back to string conversion', () => {
    expect(formatCommandError(new Error('network failed'))).toBe('network failed');
    expect(formatCommandError('plain error')).toBe('plain error');
  });
});
