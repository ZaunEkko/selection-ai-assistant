// @ts-nocheck
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const css = readFileSync(resolve(__dirname, '../styles.css'), 'utf8');

function blockFor(selector: string) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = css.match(new RegExp(`${escaped}\\s*\\{([\\s\\S]*?)\\}`));
  expect(match, `${selector} block should exist`).not.toBeNull();
  return match?.[1] ?? '';
}

describe('Settings window layout styles', () => {
  it('makes the settings window scroll within the fixed Tauri window', () => {
    const settingsWindow = blockFor('.settings-window');

    expect(settingsWindow).toMatch(/height:\s*100vh/);
    expect(settingsWindow).toMatch(/min-height:\s*0/);
    expect(settingsWindow).toMatch(/overflow-y:\s*auto/);
  });

  it('uses a structured settings layout instead of plain stacked boxes', () => {
    const settingsHero = blockFor('.settings-hero');
    const settingsGrid = blockFor('.settings-grid');
    const providerForm = blockFor('.provider-form');

    expect(settingsHero).toMatch(/border-radius:\s*24px/);
    expect(settingsGrid).toMatch(/grid-template-columns:\s*minmax\(0,\s*1\.35fr\)\s+minmax\(260px,\s*0\.65fr\)/);
    expect(providerForm).toMatch(/border-radius:\s*20px/);
  });
});


describe('AI panel scroll layout styles', () => {
  it('keeps the AI panel constrained to the window height and scrollable when manually resized smaller', () => {
    const aiPanel = blockFor('.ai-panel');

    expect(aiPanel).toMatch(/height:\s*100vh/);
    expect(aiPanel).toMatch(/min-height:\s*0/);
    expect(aiPanel).toMatch(/overflow-y:\s*auto/);
  });

  it('places the answer area in a shrinkable remaining-space grid row', () => {
    const aiPanel = blockFor('.ai-panel');

    expect(aiPanel).toMatch(/grid-template-rows:\s*auto\s+auto\s+auto\s+minmax\(0,\s*1fr\)\s+auto/);
  });

  it('makes generated answers internally scrollable', () => {
    const answer = blockFor('.ai-answer');

    expect(answer).toMatch(/min-height:\s*0/);
    expect(answer).toMatch(/overflow-y:\s*auto/);
  });

  it('styles the independent source text window instead of a same-panel sidebar', () => {
    const sourceWindow = blockFor('.source-text-window');
    const sourceBody = blockFor('.source-text-body');

    expect(sourceWindow).toMatch(/height:\s*100vh/);
    expect(sourceWindow).toMatch(/display:\s*grid/);
    expect(sourceBody).toMatch(/overflow-y:\s*auto/);
    expect(sourceBody).toMatch(/white-space:\s*pre-wrap/);
  });

  it('visually clamps selected text with a cleaner card interaction until it is expanded', () => {
    const selectedTextPreview = blockFor('.selected-text-preview');
    const collapsedSelectedText = blockFor('.selected-text-preview.is-collapsed');
    const expandedSelectedText = blockFor('.selected-text-preview.is-expanded');
    const selectedTextActions = blockFor('.selected-text-actions');
    const expandButton = blockFor('.selected-text-expand-button');

    expect(selectedTextPreview).toMatch(/overflow:\s*hidden/);
    expect(collapsedSelectedText).toMatch(/max-height:\s*4\.8em/);
    expect(collapsedSelectedText).toMatch(/mask-image:\s*linear-gradient/);
    expect(expandedSelectedText).toMatch(/max-height:\s*min\(32vh,\s*220px\)/);
    expect(expandedSelectedText).toMatch(/overflow-y:\s*auto/);
    expect(selectedTextActions).toMatch(/justify-content:\s*space-between/);
    expect(expandButton).toMatch(/border-radius:\s*999px/);
  });

  it('makes action switch buttons look like a distinct segmented tool group', () => {
    const actionBar = blockFor('.action-bar');
    const actionSwitchButton = blockFor('.action-switch-button');
    const activeActionSwitchButton = blockFor('.action-switch-button[aria-pressed="true"]');

    expect(actionBar).toMatch(/border:\s*1px\s+solid\s+rgba\(148,\s*163,\s*184,\s*0\.28\)/);
    expect(actionBar).toMatch(/background:\s*#f8fafc/);
    expect(actionSwitchButton).toMatch(/border:\s*0/);
    expect(actionSwitchButton).toMatch(/border-radius:\s*999px/);
    expect(activeActionSwitchButton).toMatch(/box-shadow:\s*0\s+8px\s+20px\s+rgba\(37,\s*99,\s*235,\s*0\.24\)/);
  });

  it('styles the execute action button as a restrained primary CTA without heavy bold or shadow', () => {
    const buttonRow = blockFor('.panel-control-buttons');
    const executeButton = blockFor('.execute-action-button');
    const executeButtonHover = blockFor('.execute-action-button:hover');

    expect(buttonRow).toMatch(/justify-content:\s*flex-end/);
    expect(executeButton).toMatch(/display:\s*inline-flex/);
    expect(executeButton).toMatch(/border:\s*1px\s+solid\s+rgba\(37,\s*99,\s*235,\s*0\.28\)/);
    expect(executeButton).toMatch(/color:\s*#1e40af/);
    expect(executeButton).toMatch(/background:\s*linear-gradient\(180deg,\s*#ffffff,\s*#f8fbff\)/);
    expect(executeButton).not.toMatch(/font-weight:\s*700/);
    expect(executeButton).not.toMatch(/box-shadow/);
    expect(executeButtonHover).toMatch(/background:\s*#eff6ff/);
    expect(executeButtonHover).not.toMatch(/transform/);
    expect(executeButtonHover).not.toMatch(/box-shadow/);
  });

  it('styles rendered markdown and preview images inside the answer area', () => {
    const markdownPreview = blockFor('.markdown-preview');
    const markdownImage = blockFor('.markdown-preview img');

    expect(markdownPreview).toMatch(/white-space:\s*normal/);
    expect(markdownImage).toMatch(/max-width:\s*100%/);
    expect(markdownImage).toMatch(/object-fit:\s*contain/);
  });

  it('draws borders around rendered markdown tables', () => {
    const tableWrap = blockFor('.markdown-table-wrap');
    const table = blockFor('.markdown-preview table');
    const tableCells = blockFor('.markdown-preview th,\n.markdown-preview td');

    expect(tableWrap).toMatch(/overflow-x:\s*auto/);
    expect(table).toMatch(/border-collapse:\s*collapse/);
    expect(tableCells).toMatch(/border:\s*1px\s+solid\s+#cbd5e1/);
  });
});
