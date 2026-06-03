import { getCurrentWindow } from '@tauri-apps/api/window';
import { openPanelFromFloatingButton } from './api/tauri';
import { AiPanel } from './windows/AiPanel';
import { FloatingButton } from './windows/FloatingButton';
import { Settings } from './windows/Settings';
import { SourceTextWindow } from './windows/SourceTextWindow';

export default function App() {
  const label = getCurrentWindow().label;

  if (label === 'floating-button') {
    return <FloatingButton onClick={() => void openPanelFromFloatingButton()} />;
  }

  if (label === 'ai-panel') {
    return <AiPanel />;
  }

  if (label === 'source-text') {
    return <SourceTextWindow />;
  }

  return <Settings />;
}
