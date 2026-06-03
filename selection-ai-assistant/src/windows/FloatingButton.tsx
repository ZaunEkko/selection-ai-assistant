type Props = {
  onClick?: () => void;
};

export function FloatingButton({ onClick }: Props) {
  return (
    <div className="floating-button-window">
      <button className="floating-ai-button" type="button" onClick={onClick} aria-label="打开 AI 助手">
        <span className="floating-ai-mark" aria-hidden="true">
          <svg className="floating-ai-mark-icon" viewBox="0 0 24 24" focusable="false">
            <path d="M12 2.75l1.65 5.15 5.1 1.6-5.1 1.65L12 16.25l-1.65-5.1-5.1-1.65 5.1-1.6L12 2.75z" />
            <path d="M18.25 14.25l.8 2.45 2.45.8-2.45.8-.8 2.45-.8-2.45-2.45-.8 2.45-.8.8-2.45z" />
          </svg>
        </span>
      </button>
    </div>
  );
}
