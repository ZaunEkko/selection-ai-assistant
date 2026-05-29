import { getCurrentWindow } from '@tauri-apps/api/window';
import { openPanelFromFloatingButton } from './api/tauri';
import { AiPanel } from './windows/AiPanel';
import { FloatingButton } from './windows/FloatingButton';
import { Settings } from './windows/Settings';

export default function App() {
  const label = getCurrentWindow().label;

  if (label === 'floating-button') {
    return <FloatingButton onClick={() => void openPanelFromFloatingButton()} />;
  }

  if (label === 'ai-panel') {
    return <AiPanel />;
  }

  return <Settings />;
}
