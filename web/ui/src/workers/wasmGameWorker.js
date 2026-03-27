import initWasm, { WasmGame } from "../../../wasm_demo/pkg/ironsmith.js";
import wasmUrl from "../../../wasm_demo/pkg/ironsmith_bg.wasm?url";

const WASM_ESTIMATED_SIZE = 12_500_000; // ~12MB fallback estimate

let game = null;
let callQueue = Promise.resolve();
let pendingCallCount = 0;
let backgroundCompileDone = false;
let backgroundCompileTimer = null;
let lastRegistryLoaded = -1;
let lastRegistryTotal = -1;
const SNAPSHOT_METHODS = new Set([
  "advancePhase",
  "cancelDecision",
  "dispatch",
  "snapshot",
  "startMatch",
  "switchPerspective",
  "uiState",
]);
const DISPATCH_TRACE_METHODS = new Set([
  "advancePhase",
  "cancelDecision",
  "dispatch",
]);

function nowMs() {
  return performance.now();
}

function clampMs(value) {
  return Number.isFinite(value) ? Math.max(0, value) : 0;
}

function decorateResultWithPerf(result, perf) {
  if (!result || typeof result !== "object" || Array.isArray(result)) {
    return result;
  }
  return {
    ...result,
    __perf: perf,
  };
}

function serializeError(err) {
  if (err instanceof Error) {
    return {
      name: err.name,
      message: err.message,
      stack: err.stack,
    };
  }
  return {
    name: "Error",
    message: String(err),
  };
}

function postProgress(phase, progress) {
  self.postMessage({ type: "progress", phase, progress });
}

function normalizeRegistryStatus(raw) {
  const loaded = Number(raw?.loaded ?? 0);
  const total = Number(raw?.total ?? 0);
  const done = Boolean(raw?.done);
  return {
    loaded: Number.isFinite(loaded) ? Math.max(0, Math.floor(loaded)) : 0,
    total: Number.isFinite(total) ? Math.max(0, Math.floor(total)) : 0,
    done,
  };
}

function postRegistryStatus(raw, force = false) {
  const status = normalizeRegistryStatus(raw);
  if (
    !force
    && status.loaded === lastRegistryLoaded
    && status.total === lastRegistryTotal
  ) {
    return;
  }
  lastRegistryLoaded = status.loaded;
  lastRegistryTotal = status.total;
  self.postMessage({
    type: "registry",
    loaded: status.loaded,
    total: status.total,
    done: status.done,
  });
}

function clearBackgroundTimer() {
  if (backgroundCompileTimer !== null) {
    self.clearTimeout(backgroundCompileTimer);
    backgroundCompileTimer = null;
  }
}

function scheduleBackgroundCompile(delay = 0) {
  if (backgroundCompileDone || !game || typeof game.preloadRegistryChunk !== "function") {
    return;
  }
  if (backgroundCompileTimer !== null) return;
  backgroundCompileTimer = self.setTimeout(async () => {
    backgroundCompileTimer = null;
    await runBackgroundCompileStep();
  }, delay);
}

async function runBackgroundCompileStep() {
  if (backgroundCompileDone || !game || typeof game.preloadRegistryChunk !== "function") {
    return;
  }
  if (pendingCallCount > 0) {
    scheduleBackgroundCompile(32);
    return;
  }
  try {
    const status = await game.preloadRegistryChunk(16);
    postRegistryStatus(status);
    if (status?.done) {
      backgroundCompileDone = true;
      return;
    }
  } catch (err) {
    self.postMessage({ type: "error", error: serializeError(err) });
    return;
  }
  scheduleBackgroundCompile(16);
}

async function fetchWasmWithProgress(url, onProgress) {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) throw new Error(`WASM fetch failed: HTTP ${response.status}`);

  const contentLength = response.headers.get("content-length");
  const parsedTotal = contentLength ? Number.parseInt(contentLength, 10) : NaN;
  const total =
    Number.isFinite(parsedTotal) && parsedTotal > 0
      ? parsedTotal
      : WASM_ESTIMATED_SIZE;

  if (!response.body) {
    const body = await response.arrayBuffer();
    onProgress(1);
    return {
      wasmResponse: new Response(body, {
        headers: { "content-type": "application/wasm" },
      }),
      downloadDone: Promise.resolve(),
    };
  }

  const [progressBody, wasmBody] = response.body.tee();

  const downloadDone = (async () => {
    const reader = progressBody.getReader();
    let received = 0;
    let lastReported = 0;

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      received += value.byteLength;
      const next = Math.min(received / total, 1);
      if (next - lastReported >= 0.005 || next === 1) {
        onProgress(next);
        lastReported = next;
      }
    }
    onProgress(1);
  })();

  return {
    wasmResponse: new Response(wasmBody, {
      headers: { "content-type": "application/wasm" },
    }),
    downloadDone,
  };
}

async function handleInit() {
  try {
    clearBackgroundTimer();
    game = null;
    pendingCallCount = 0;
    backgroundCompileDone = false;
    lastRegistryLoaded = -1;
    lastRegistryTotal = -1;
    postProgress("module", 0);

    postProgress("download", 0);
    const bust = `v=${Date.now()}-${Math.floor(Math.random() * 1e9)}`;
    const { wasmResponse, downloadDone } = await fetchWasmWithProgress(
      `${wasmUrl}?${bust}`,
      (p) => postProgress("download", p)
    );

    await downloadDone;
    postProgress("init", 1);
    await initWasm(wasmResponse);
    game = new WasmGame();
    if (typeof game.preloadRegistryStatus === "function") {
      const status = await game.preloadRegistryStatus();
      postRegistryStatus(status, true);
      backgroundCompileDone = Boolean(status?.done);
      if (!backgroundCompileDone) {
        scheduleBackgroundCompile(0);
      }
    }

    self.postMessage({ type: "ready" });
  } catch (err) {
    self.postMessage({ type: "error", error: serializeError(err) });
  }
}

function enqueueCall(task) {
  callQueue = callQueue.then(task, task);
  return callQueue;
}

function handleCall(msg) {
  const { id, method, args = [] } = msg;
  const enqueuedAt = nowMs();
  pendingCallCount += 1;
  enqueueCall(async () => {
    if (!game) throw new Error("Game is not initialized yet");
    const startedAt = nowMs();
    const queueWaitMs = startedAt - enqueuedAt;
    const fn = game[method];
    if (typeof fn !== "function") {
      throw new Error(`Unknown game method: ${method}`);
    }
    const wasmStartedAt = nowMs();
    const result = await fn.apply(game, args);
    const wasmCallMs = nowMs() - wasmStartedAt;
    let snapshotPerf = null;
    let snapshotPerfReadMs = 0;
    let dispatchPerf = null;
    let dispatchPerfReadMs = 0;
    let replayExecutionPerf = null;
    let replayExecutionPerfReadMs = 0;
    let advanceUntilDecisionPerf = null;
    let advanceUntilDecisionPerfReadMs = 0;
    if (SNAPSHOT_METHODS.has(method)) {
      const snapshotPerfStartedAt = nowMs();
      snapshotPerf = typeof game.lastSnapshotPerf === "function"
        ? await game.lastSnapshotPerf()
        : null;
      snapshotPerfReadMs = nowMs() - snapshotPerfStartedAt;
    }
    if (DISPATCH_TRACE_METHODS.has(method)) {
      const dispatchPerfStartedAt = nowMs();
      dispatchPerf = typeof game.lastDispatchPerf === "function"
        ? await game.lastDispatchPerf()
        : null;
      dispatchPerfReadMs = nowMs() - dispatchPerfStartedAt;
      const replayExecutionPerfStartedAt = nowMs();
      replayExecutionPerf = typeof game.lastReplayExecutionPerf === "function"
        ? await game.lastReplayExecutionPerf()
        : null;
      replayExecutionPerfReadMs = nowMs() - replayExecutionPerfStartedAt;
      const advanceUntilDecisionPerfStartedAt = nowMs();
      advanceUntilDecisionPerf = typeof game.lastAdvanceUntilDecisionPerf === "function"
        ? await game.lastAdvanceUntilDecisionPerf()
        : null;
      advanceUntilDecisionPerfReadMs = nowMs() - advanceUntilDecisionPerfStartedAt;
    }
    const registryStatusStartedAt = nowMs();
    let registryStatus = null;
    if (typeof game.preloadRegistryStatus === "function") {
      registryStatus = await game.preloadRegistryStatus();
    }
    const registryStatusMs = nowMs() - registryStatusStartedAt;
    const totalWorkerMs = nowMs() - enqueuedAt;
    const snapshotTotalMs = Number(snapshotPerf?.totalSnapshotMs ?? 0);
    const perf = {
      method,
      queueWaitMs: clampMs(queueWaitMs),
      wasmCallMs: clampMs(wasmCallMs),
      snapshotPerfReadMs: clampMs(snapshotPerfReadMs),
      dispatchPerfReadMs: clampMs(dispatchPerfReadMs),
      replayExecutionPerfReadMs: clampMs(replayExecutionPerfReadMs),
      advanceUntilDecisionPerfReadMs: clampMs(advanceUntilDecisionPerfReadMs),
      registryStatusMs: clampMs(registryStatusMs),
      totalWorkerMs: clampMs(totalWorkerMs),
      estimatedEngineMs: clampMs(wasmCallMs - snapshotTotalMs),
      snapshot: snapshotPerf || null,
      dispatch: dispatchPerf || null,
      replayExecution: replayExecutionPerf || null,
      advanceUntilDecision: advanceUntilDecisionPerf || null,
    };
    return {
      result: decorateResultWithPerf(result, perf),
      registryStatus,
    };
  })
    .then(({ result, registryStatus }) => {
      if (registryStatus) {
        postRegistryStatus(registryStatus);
        if (!registryStatus.done) scheduleBackgroundCompile(0);
      }
      self.postMessage({ type: "result", id, ok: true, result });
    })
    .catch((err) => {
      self.postMessage({
        type: "result",
        id,
        ok: false,
        error: serializeError(err),
      });
    })
    .finally(() => {
      pendingCallCount = Math.max(0, pendingCallCount - 1);
      if (!backgroundCompileDone) {
        scheduleBackgroundCompile(0);
      }
    });
}

self.addEventListener("message", (event) => {
  const msg = event.data || {};
  if (msg.type === "init") {
    handleInit(msg);
    return;
  }
  if (msg.type === "call") {
    handleCall(msg);
  }
});
