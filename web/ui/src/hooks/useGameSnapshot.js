import { startTransition, useCallback, useLayoutEffect, useRef, useState } from 'react';

// Engine state becomes authoritative immediately, but drawing a whole table
// must yield to pointer movement and animation frames. Never copy a delayed
// rendered snapshot back into the command path.
export function useGameSnapshot(initialState = null) {
  const [state, renderState] = useState(initialState);
  const stateRef = useRef(state);
  const renderedStateRef = useRef(state);
  useLayoutEffect(() => { renderedStateRef.current = state; }, [state]);
  const isSnapshotRendered = useCallback(() => renderedStateRef.current === stateRef.current, []);
  const listeners = useRef(new Set());
  const setState = useCallback(next => {
    const snapshot = typeof next === 'function' ? next(stateRef.current) : next;
    stateRef.current = snapshot;
    for (const listener of listeners.current) listener(snapshot);
    startTransition(() => renderState(snapshot));
  }, []);
  const subscribeState = useCallback(listener => {
    listeners.current.add(listener);
    listener(stateRef.current);
    return () => listeners.current.delete(listener);
  }, []);
  return { state, setState, stateRef, subscribeState, isSnapshotRendered };
}
