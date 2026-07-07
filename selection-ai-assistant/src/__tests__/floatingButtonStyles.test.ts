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

  it('keeps the Tauri floating window large enough for the horizontal pill action bar', () => {
    const css = readFileSync(resolve(__dirname, '../styles.css'), 'utf8');
    const tauriConfig = JSON.parse(readFileSync(resolve(__dirname, '../../src-tauri/tauri.conf.json'), 'utf8'));
    const panelRs = readFileSync(resolve(__dirname, '../../src-tauri/src/commands/panel.rs'), 'utf8');

    const floatingWindow = tauriConfig.app.windows.find((window: { label?: string }) => window.label === 'floating-button');

    expect(floatingWindow).toMatchObject({ width: 244, height: 44, shadow: false });
    expect(css).toMatch(/\.mini-action-bar[\s\S]*min-width:\s*228px/);
    expect(css).toMatch(/\.mini-action-bar[\s\S]*min-height:\s*34px/);
    expect(css).toMatch(/\.mini-action-button[\s\S]*padding:\s*6px\s+11px/);
    expect(panelRs).toMatch(
      /const\s+FLOATING_BUTTON_SIZE:\s*WindowSize\s*=\s*WindowSize\s*\{[\s\S]*width:\s*244\.0,[\s\S]*height:\s*44\.0,[\s\S]*\}/,
    );
  });

  it('keeps the translation result window compact, resizable, and single-column', () => {
    const css = readFileSync(resolve(__dirname, '../styles.css'), 'utf8');
    const tauriConfig = JSON.parse(readFileSync(resolve(__dirname, '../../src-tauri/tauri.conf.json'), 'utf8'));
    const panelRs = readFileSync(resolve(__dirname, '../../src-tauri/src/commands/panel.rs'), 'utf8');
    const capabilities = JSON.parse(readFileSync(resolve(__dirname, '../../src-tauri/capabilities/default.json'), 'utf8'));

    const translateWindow = tauriConfig.app.windows.find((window: { label?: string }) => window.label === 'translate-result');

    expect(translateWindow).toMatchObject({
      width: 320,
      height: 180,
      minWidth: 260,
      minHeight: 140,
      resizable: true,
      shadow: false,
    });
    expect(css).toMatch(/\.translate-result-window[\s\S]*min-width:\s*260px/);
    expect(css).toMatch(/\.translate-result-text[\s\S]*overflow-y:\s*auto/);
    expect(css).toMatch(/\.translate-result-resize[\s\S]*cursor:\s*nwse-resize/);
    expect(css).not.toMatch(/\.translate-result-compare[\s\S]*grid-template-columns:\s*1fr\s+1fr/);
    expect(capabilities.permissions).toContain('core:window:allow-start-resize-dragging');
    expect(panelRs).toMatch(
      /const\s+TRANSLATE_RESULT_SIZE:\s*WindowSize\s*=\s*WindowSize\s*\{[\s\S]*width:\s*320\.0,[\s\S]*height:\s*180\.0,[\s\S]*\}/,
    );
  });
});
