const DEFAULT_ENGINE_WORKER_TIMEOUT_MS = 15_000;
const MIN_ENGINE_WORKER_TIMEOUT_MS = 5_000;
const MAX_ENGINE_WORKER_TIMEOUT_MS = 120_000;

const WATCHED_ENGINE_METHODS = new Set([
  "addCardToHand",
  "addCardToZone",
  "addCardsToZones",
  "addLifeDelta",
  "advancePhase",
  "applyVerifiedHiddenLibraryShuffle",
  "cancelDecision",
  "dispatch",
  "drawCard",
  "exportPublicAuditCheckpoint",
  "exportRedactedSyncCheckpoint",
  "exportSyncCheckpoint",
  "forfeitPlayer",
  "importSyncCheckpoint",
  "injectTranscriptRandomSeeds",
  "reset",
  "resetEmpty",
  "revealHiddenObject",
  "revealHiddenPosition",
  "revealHiddenPositions",
  "revealHiddenSlot",
  "setLife",
  "setPerspective",
  "snapshot",
  "snapshotJson",
  "switchPerspective",
  "uiState",
]);

const FATAL_WASM_ERROR = /(?:\bRuntimeError\b|unreachable|memory access out of bounds|out of memory|allocation failed|call stack|stack overflow|wasm trap)/i;

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

export function isFatalWasmWorkerError(raw) {
  const name = String(raw?.name || "");
  const message = String(raw?.message || raw || "");
  return FATAL_WASM_ERROR.test(`${name}: ${message}`);
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

export function makeFatalWasmWorkerError(raw, checkpointAvailable = false) {
  const source = raw instanceof Error ? raw : new Error(String(raw?.message || raw || "WASM trap"));
  const error = new Error(`WASM runtime failed and was restarted: ${source.message}`);
  error.name = "EngineWorkerRuntimeError";
  error.code = "ENGINE_WORKER_RUNTIME_FAILURE";
  error.engineRecoveryAttempted = true;
  error.engineCheckpointAvailable = Boolean(checkpointAvailable);
  error.requiresAuthoritativeResync = true;
  return error;
}
