import type { UiAction } from '../api/tauri';
import { actionOptions } from './actionLabels';

type Props = {
  activeAction: UiAction;
  onSelect: (action: UiAction) => void;
};

export function ActionBar({ activeAction, onSelect }: Props) {
  return (
    <div className="action-bar" aria-label="AI 操作">
      {actionOptions.map(([action, label]) => (
        <button key={action} type="button" aria-pressed={activeAction === action} onClick={() => onSelect(action)}>
          {label}
        </button>
      ))}
    </div>
  );
}
