type Props = {
  onClick?: () => void;
};

export function FloatingButton({ onClick }: Props) {
  return (
    <button className="floating-ai-button" type="button" onClick={onClick} aria-label="Open AI assistant">
      AI
    </button>
  );
}
