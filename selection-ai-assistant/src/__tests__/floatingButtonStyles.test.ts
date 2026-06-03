// @ts-nocheck
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('floating button styles', () => {
  it('keeps floating window transparent and avoids large clipped shadows', () => {
    const css = readFileSync(resolve(__dirname, '../styles.css'), 'utf8');

    expect(css).toMatch(/html,\s*body,\s*#root[\s\S]*background:\s*transparent/);
    expect(css).toMatch(/html,\s*body,\s*#root[\s\S]*border:\s*0/);
    expect(css).toMatch(/html,\s*body,\s*#root[\s\S]*outline:\s*0/);
    expect(css).toMatch(/html,\s*body,\s*#root[\s\S]*box-shadow:\s*none/);
    expect(css).toMatch(/\.floating-button-window[\s\S]*background:\s*transparent/);
    expect(css).toMatch(/\.floating-button-window[\s\S]*border:\s*0/);
    expect(css).toMatch(/\.floating-button-window[\s\S]*outline:\s*0/);
    expect(css).toMatch(/\.floating-button-window[\s\S]*box-shadow:\s*none/);
    expect(css).not.toMatch(/box-shadow:\s*0\s+10px\s+24px/);
    expect(css).not.toMatch(/\.floating-ai-button[\s\S]*box-shadow:\s*\n\s*0\s+2px\s+8px/);
  });

  it('keeps the Tauri floating window the same size as the round CSS button', () => {
    const css = readFileSync(resolve(__dirname, '../styles.css'), 'utf8');
    const tauriConfig = JSON.parse(readFileSync(resolve(__dirname, '../../src-tauri/tauri.conf.json'), 'utf8'));
    const panelRs = readFileSync(resolve(__dirname, '../../src-tauri/src/commands/panel.rs'), 'utf8');

    const floatingWindow = tauriConfig.app.windows.find((window: { label?: string }) => window.label === 'floating-button');

    expect(floatingWindow).toMatchObject({ width: 40, height: 40, shadow: false });
    expect(css).toMatch(/\.floating-ai-button[\s\S]*width:\s*40px/);
    expect(css).toMatch(/\.floating-ai-button[\s\S]*height:\s*40px/);
    expect(panelRs).toMatch(
      /const\s+FLOATING_BUTTON_SIZE:\s*WindowSize\s*=\s*WindowSize\s*\{[\s\S]*width:\s*40\.0,[\s\S]*height:\s*40\.0,[\s\S]*\}/,
    );
  });
});
