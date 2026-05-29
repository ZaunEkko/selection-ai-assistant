import { getCurrentWindow } from '@tauri-apps/api/window';
import { AiPanel } from './windows/AiPanel';
import { FloatingButton } from './windows/FloatingButton';
import { Settings } from './windows/Settings';

export default function App() {
  const label = getCurrentWindow().label;

  if (label === 'floating-button') {
    return <FloatingButton />;
  }

  if (label === 'ai-panel') {
    return <AiPanel />;
  }

  return <Settings />;
}
