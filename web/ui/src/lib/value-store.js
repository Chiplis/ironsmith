export function createValueStore(initial = null) {
  let value = initial;
  const listeners = new Set();
  return {
    getSnapshot: () => value,
    subscribe(listener) { listeners.add(listener); return () => listeners.delete(listener); },
    set(next) {
      if (Object.is(value, next)) return;
      value = next;
      for (const listener of listeners) listener();
    },
  };
}
export function differsBeyondClock(before, after) {
  const ignored = new Set(['matchClock', 'actionTimer']);
  return Object.keys({ ...before, ...after }).some(key => !ignored.has(key) && before[key] !== after[key]);
}
