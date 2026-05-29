import { describe, expect, it } from 'vitest';
import { createPanelState, initialPanelState, panelReducer } from '../stores/panelStore';

describe('panel store reducer', () => {
  it('ignores stream deltas from stale request ids', () => {
    const state = createPanelState();

    state.startRequest('request-1');
    state.appendDelta('request-1', 'hello');
    state.startRequest('request-2');
    state.appendDelta('request-1', ' stale');
    state.appendDelta('request-2', 'new');

    expect(state.answer).toBe('new');
  });

  it('ignores stale finish events and only clears the active request', () => {
    const state = createPanelState();

    state.startRequest('request-1');
    state.startRequest('request-2');
    state.finishRequest('request-1');

    expect(state.running).toBe(true);
    expect(state.activeRequestId).toBe('request-2');

    state.finishRequest('request-2');

    expect(state.running).toBe(false);
    expect(state.activeRequestId).toBeNull();
  });

  it('reduces stale deltas and stale finishes without mutating previous snapshots', () => {
    const request1 = panelReducer(initialPanelState, { type: 'start', requestId: 'request-1' });
    const withDelta = panelReducer(request1, { type: 'delta', requestId: 'request-1', delta: 'hello' });
    const request2 = panelReducer(withDelta, { type: 'start', requestId: 'request-2' });
    const afterStaleDelta = panelReducer(request2, { type: 'delta', requestId: 'request-1', delta: ' stale' });
    const afterNewDelta = panelReducer(afterStaleDelta, { type: 'delta', requestId: 'request-2', delta: 'new' });
    const afterStaleFinish = panelReducer(afterNewDelta, { type: 'finish', requestId: 'request-1' });

    expect(afterNewDelta.answer).toBe('new');
    expect(afterStaleFinish).toEqual({ activeRequestId: 'request-2', answer: 'new', running: true });
    expect(initialPanelState).toEqual({ activeRequestId: null, answer: '', running: false });
  });

  it('keeps createPanelState methods bound when calls are destructured', () => {
    const state = createPanelState();
    const { startRequest, appendDelta, finishRequest, reset } = state;

    startRequest('request-1');
    appendDelta('request-1', 'hello');
    finishRequest('stale-request');

    expect(state.answer).toBe('hello');
    expect(state.running).toBe(true);

    finishRequest('request-1');
    expect(state.running).toBe(false);

    reset();
    expect(state.answer).toBe('');
    expect(state.activeRequestId).toBeNull();
  });
});
