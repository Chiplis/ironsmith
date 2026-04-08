export const DEFAULT_WASM_INTERACTION_DEBOUNCE_MS = 100;

function defaultNow() {
  if (typeof performance !== "undefined" && typeof performance.now === "function") {
    return performance.now();
  }
  return Date.now();
}

export function createWasmInteractionGate({
  debounceMs = DEFAULT_WASM_INTERACTION_DEBOUNCE_MS,
  now = defaultNow,
} = {}) {
  let inFlight = false;
  let cooldownUntil = -Infinity;

  const isBlocked = () => inFlight || now() < cooldownUntil;

  const run = async (task) => {
    if (typeof task !== "function" || isBlocked()) {
      return undefined;
    }

    inFlight = true;
    try {
      return await task();
    } finally {
      inFlight = false;
      cooldownUntil = now() + debounceMs;
    }
  };

  return {
    isBlocked,
    run,
  };
}
