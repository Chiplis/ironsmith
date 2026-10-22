// Arrival lifetimes include planning, flight, and the final landing flourish.
const pending = new Set();
const listeners = new Set();
const publish = () => listeners.forEach(listener => listener());
export const stackArrivalActivity = {
  subscribe(listener) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
  getSnapshot: () => pending.size,
};

export function beginStackArrival() {
  const token = {};
  pending.add(token);
  publish();
  return () => {
    if (pending.delete(token)) publish();
  };
}
