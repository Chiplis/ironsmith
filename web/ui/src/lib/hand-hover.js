import { useSyncExternalStore } from "react";

// The hand card under the pointer (or keyboard focus), shared so a decision
// panel listing hand cards can shade that card's row while it is hovered.
let current = null;
const listeners = new Set();

export function setHandHoverObjectId(objectId) {
  const next = objectId == null ? null : String(objectId);
  if (next === current) return;
  current = next;
  for (const listener of listeners) listener();
}

function subscribe(listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useHandHoverObjectId() {
  return useSyncExternalStore(subscribe, () => current, () => null);
}
