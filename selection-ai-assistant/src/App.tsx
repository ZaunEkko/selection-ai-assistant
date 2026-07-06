import { getCurrentWindow } from '@tauri-apps/api/window';
import { AiPanel } from './windows/AiPanel';
import { MiniActionBar } from './windows/MiniActionBar';
import { Settings } from './windows/Settings';
import { SourceTextWindow } from './windows/SourceTextWindow';
import { TranslateResult } from './windows/TranslateResult';

export default function App() {
  const label = getCurrentWindow().label;

  if (label === 'floating-button') {
    return <MiniActionBar />;
  }

  if (label === 'ai-panel') {
    return <AiPanel />;
  }

  if (label === 'source-text') {
    return <SourceTextWindow />;
  }

  if (label === 'translate-result') {
    return <TranslateResult />;
  }

  return <Settings />;
}
