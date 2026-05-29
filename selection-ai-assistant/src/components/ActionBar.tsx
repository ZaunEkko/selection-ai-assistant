import type { UiAction } from '../api/tauri';

type Props = {
  activeAction: UiAction;
  onRun: (action: UiAction) => void;
};

const actions: Array<[UiAction, string]> = [
  ['translateExplain', '翻译解释'],
  ['explain', '解释'],
  ['summarize', '总结'],
  ['codeExplain', '代码解释'],
  ['errorExplain', '报错解释'],
];

export function ActionBar({ activeAction, onRun }: Props) {
  return (
    <div className="action-bar" aria-label="AI actions">
      {actions.map(([action, label]) => (
        <button key={action} type="button" aria-pressed={activeAction === action} onClick={() => onRun(action)}>
          {label}
        </button>
      ))}
    </div>
  );
}
