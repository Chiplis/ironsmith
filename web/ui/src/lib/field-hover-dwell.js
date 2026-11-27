// Pointer dwell over a battlefield card. A hover preview should not chase a
// pointer that is merely crossing the board, so the battlefield reports which
// card the pointer is still travelling over (`pending`) and which card it has
// come to rest on (`dwelled`). The preview waits out the first and opens at
// once for the second.
import { useSyncExternalStore } from "react";

let snapshot = { pending: null, dwelled: null };
const listeners = new Set();

export function setFieldHoverDwell(next) {
  const pending = next?.pending == null ? null : String(next.pending);
  const dwelled = next?.dwelled == null ? null : String(next.dwelled);
  if (snapshot.pending === pending && snapshot.dwelled === dwelled) return;
  snapshot = { pending, dwelled };
  for (const listener of listeners) listener();
}

function subscribe(listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

const read = () => snapshot;

export function useFieldHoverDwell() {
  return useSyncExternalStore(subscribe, read, read);
}
