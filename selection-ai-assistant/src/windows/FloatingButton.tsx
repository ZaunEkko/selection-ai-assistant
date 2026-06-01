type Props = {
  onClick?: () => void;
};

export function FloatingButton({ onClick }: Props) {
  return (
    <button className="floating-ai-button" type="button" onClick={onClick} aria-label="打开 AI 助手">
      AI
    </button>
  );
}
