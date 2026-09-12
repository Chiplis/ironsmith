// Pointer samples are immediately readable at release, but visual subscribers
// are notified at most once per frame.
export function createFrameStore(schedule = requestAnimationFrame, cancel = cancelAnimationFrame) {
  let value = null;
  let frame = null;
  const listeners = new Set();
  const flush = () => {
    frame = null;
    listeners.forEach((listener) => listener());
  };
  return {
    getSnapshot: () => value,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    set(next) {
      if (Object.is(value, next)) return;
      value = next;
      if (frame === null) frame = schedule(flush);
    },
    dispose() {
      if (frame !== null) cancel(frame);
      frame = null;
      listeners.clear();
    },
  };
}
