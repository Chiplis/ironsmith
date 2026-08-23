import initWasm, {
  ziffleBuildRevealToken,
  ziffleBuildRevealTokens,
  ziffleBuildShuffleStep,
  ziffleKeygen,
  ziffleRevealCard,
  ziffleRevealCards,
  ziffleVerifyShuffle,
} from "../../../wasm_demo/pkg/verifier.js";
import wasmUrl from "../../../wasm_demo/pkg/verifier_bg.wasm?url";

const verifier = {
  ziffleBuildRevealToken,
  ziffleBuildRevealTokens,
  ziffleBuildShuffleStep,
  ziffleKeygen,
  ziffleRevealCard,
  ziffleRevealCards,
  ziffleVerifyShuffle,
};

const ZIFFLE_METHODS = new Set([
  "ziffleBuildRevealToken",
  "ziffleBuildRevealTokens",
  "ziffleBuildShuffleStep",
  "ziffleKeygen",
  "ziffleRevealCard",
  "ziffleRevealCards",
  "ziffleVerifyShuffle",
]);

let game = null;
let initPromise = null;
let workerIndex = 0;

function nowMs() {
  return performance.now();
}

function clampMs(value) {
  return Number.isFinite(value) ? Math.max(0, value) : 0;
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

function decorateResultWithPerf(result, perf) {
  if (!result || typeof result !== "object" || Array.isArray(result)) {
    return result;
  }
  return {
    ...result,
    __perf: perf,
  };
}

async function ensureReady() {
  if (game) return game;
  if (!initPromise) {
    initPromise = (async () => {
      await initWasm(wasmUrl);
      game = verifier;
      return game;
    })();
  }
  return initPromise;
}

async function handleInit(msg) {
  try {
    workerIndex = Number.isFinite(Number(msg.workerIndex))
      ? Math.max(0, Math.floor(Number(msg.workerIndex)))
      : 0;
    await ensureReady();
    self.postMessage({ type: "ready", workerIndex });
  } catch (err) {
    self.postMessage({ type: "error", workerIndex, error: serializeError(err) });
  }
}

async function handleCall(msg) {
  const { id, method, args = [] } = msg;
  const enqueuedAt = nowMs();
  try {
    const readyGame = await ensureReady();
    if (!ZIFFLE_METHODS.has(method)) {
      throw new Error(`Unknown ziffle worker method: ${method}`);
    }
    const fn = readyGame[method];
    if (typeof fn !== "function") {
      throw new Error(`Unavailable ziffle worker method: ${method}`);
    }
    const startedAt = nowMs();
    const result = await fn.apply(readyGame, args);
    const perf = {
      method,
      workerIndex,
      queueWaitMs: clampMs(startedAt - enqueuedAt),
      wasmCallMs: clampMs(nowMs() - startedAt),
      totalWorkerMs: clampMs(nowMs() - enqueuedAt),
    };
    self.postMessage({
      type: "result",
      id,
      ok: true,
      result: decorateResultWithPerf(result, perf),
    });
  } catch (err) {
    self.postMessage({
      type: "result",
      id,
      ok: false,
      error: serializeError(err),
    });
  }
}

self.addEventListener("message", (event) => {
  const msg = event.data || {};
  if (msg.type === "init") {
    void handleInit(msg);
    return;
  }
  if (msg.type === "call") {
    void handleCall(msg);
  }
});
