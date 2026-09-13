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

  const run = async (task, { automatic = false } = {}) => {
    if (typeof task !== "function" || inFlight || (!automatic && isBlocked())) {
      return undefined;
    }

    inFlight = true;
    try {
      return await task();
    } finally {
      inFlight = false;
      if (!automatic) cooldownUntil = now() + debounceMs;
    }
  };

  return {
    isBlocked,
    isInFlight: () => inFlight,
    run,
    // Continuations are not new clicks. Serialize them without waiting for or
    // extending the user-input cooldown.
    runAutomatic: (task) => run(task, { automatic: true }),
    async runWhenReady(task, isCurrent = () => true) {
      while (isCurrent()) {
        if (!isBlocked()) return run(task);
        await new Promise((resolve) => setTimeout(resolve, 25));
      }
      return undefined;
    },
  };
}
