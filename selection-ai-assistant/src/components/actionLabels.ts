import type { UiAction } from '../api/tauri';

export const actionLabels: Record<UiAction, string> = {
  translateExplain: '翻译解释',
  explain: '解释',
  summarize: '总结',
  codeExplain: '代码解释',
  errorExplain: '报错解释',
  expandPrompt: '扩写提示词',
  menuFallback: 'AI 解释',
};

export const actionOptions: Array<[UiAction, string]> = [
  ['translateExplain', actionLabels.translateExplain],
  ['explain', actionLabels.explain],
  ['summarize', actionLabels.summarize],
  ['expandPrompt', actionLabels.expandPrompt],
  ['codeExplain', actionLabels.codeExplain],
  ['errorExplain', actionLabels.errorExplain],
];
