const DEFAULT_ENGINE_WORKER_TIMEOUT_MS = 15_000;
const MIN_ENGINE_WORKER_TIMEOUT_MS = 5_000;
const MAX_ENGINE_WORKER_TIMEOUT_MS = 120_000;

const WATCHED_ENGINE_METHODS = new Set([
  "advancePhase",
  "cancelDecision",
  "dispatch",
  "forfeitPlayer",
]);

export function engineWorkerTimeoutMs(configuredValue) {
  const configured = Number(configuredValue);
  if (!Number.isFinite(configured) || configured <= 0) {
    return DEFAULT_ENGINE_WORKER_TIMEOUT_MS;
  }
  return Math.max(
    MIN_ENGINE_WORKER_TIMEOUT_MS,
    Math.min(MAX_ENGINE_WORKER_TIMEOUT_MS, Math.floor(configured))
  );
}

export function shouldWatchEngineMethod(method) {
  return WATCHED_ENGINE_METHODS.has(String(method || ""));
}

export function makeEngineWorkerStallError(method, timeoutMs, checkpointAvailable = false) {
  const error = new Error(
    `Engine call ${method} exceeded ${timeoutMs} ms; the isolated runtime was restarted`
  );
  error.name = "EngineWorkerStallError";
  error.code = "ENGINE_WORKER_STALL";
  error.engineRecoveryAttempted = true;
  error.engineCheckpointAvailable = Boolean(checkpointAvailable);
  error.requiresAuthoritativeResync = true;
  return error;
}
