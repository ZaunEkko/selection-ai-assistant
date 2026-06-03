export type PanelSnapshot = {
  activeRequestId: string | null;
  answer: string;
  running: boolean;
};

export type PanelEvent =
  | { type: 'start'; requestId: string; initialAnswer?: string }
  | { type: 'delta'; requestId: string; delta: string }
  | { type: 'finish'; requestId: string }
  | { type: 'reset' };

export const initialPanelState: PanelSnapshot = {
  activeRequestId: null,
  answer: '',
  running: false,
};

export function panelReducer(state: PanelSnapshot, event: PanelEvent): PanelSnapshot {
  switch (event.type) {
    case 'start':
      return {
        activeRequestId: event.requestId,
        answer: event.initialAnswer ?? '',
        running: true,
      };
    case 'delta':
      if (state.activeRequestId !== event.requestId) return state;
      return {
        ...state,
        answer: state.answer + event.delta,
      };
    case 'finish':
      if (state.activeRequestId !== event.requestId) return state;
      return {
        ...state,
        activeRequestId: null,
        running: false,
      };
    case 'reset':
      return initialPanelState;
    default: {
      const exhaustive: never = event;
      return exhaustive;
    }
  }
}

export type PanelState = PanelSnapshot & {
  startRequest: (requestId: string) => void;
  appendDelta: (requestId: string, delta: string) => void;
  finishRequest: (requestId: string) => void;
  reset: () => void;
};

export function createPanelState(): PanelState {
  const state = {
    ...initialPanelState,
  } as PanelState;

  function apply(event: PanelEvent) {
    const next = panelReducer(state, event);
    state.activeRequestId = next.activeRequestId;
    state.answer = next.answer;
    state.running = next.running;
  }

  state.startRequest = (requestId: string) => apply({ type: 'start', requestId });
  state.appendDelta = (requestId: string, delta: string) => apply({ type: 'delta', requestId, delta });
  state.finishRequest = (requestId: string) => apply({ type: 'finish', requestId });
  state.reset = () => apply({ type: 'reset' });

  return state;
}
