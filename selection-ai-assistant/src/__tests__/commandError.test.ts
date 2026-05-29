import { describe, expect, it } from 'vitest';
import { formatCommandError } from '../api/tauri';

describe('formatCommandError', () => {
  it('uses message from Tauri public error objects', () => {
    expect(formatCommandError({ code: 'api_key_missing', message: 'Set SELECTION_AI_API_KEY before running an AI action.' })).toBe(
      'Set SELECTION_AI_API_KEY before running an AI action.',
    );
  });

  it('uses Error messages and falls back to string conversion', () => {
    expect(formatCommandError(new Error('network failed'))).toBe('network failed');
    expect(formatCommandError('plain error')).toBe('plain error');
  });
});
