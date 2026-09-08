import {
  beginEngineRequest,
  endEngineRequest,
  recordDiagnosticEvent,
} from '../lib/action-diagnostics.js';
import {
  engineWorkerTimeoutMs,
  makeEngineWorkerStallError,
  shouldWatchEngineMethod,
} from "../lib/engine-worker-watchdog.js";
import { useEffect, useRef, useState } from "react";

const MIN_INIT_PHASE_MS = 180;

const WORKER_METHODS = [
  "addCardToHand",
  "autocompleteCardNames",
  "addCardToZone",
  "addCardsToZones",
  "addLifeDelta",
  "advancePhase",
  "applyVerifiedHiddenLibraryShuffle",
  "cancelDecision",
  "cardLoadDiagnostics",
  "cardsMeetingThreshold",
  "createCustomCard",
  "dispatch",
  "drawCard",
  "drawOpeningHands",
  "exportHiddenCardOpening",
  "exportPublicAuditCheckpoint",
  "exportRedactedSyncCheckpoint",
  "exportSyncCheckpoint",
  "filterKnownCardNames",
  "finishPuzzleSetup",
  "forfeitPlayer",
  "getCardSemanticScore",
  "importSyncCheckpoint",
  "isKnownCardName",
  "lastAdvanceUntilDecisionPerf",
  "lastDispatchPerf",
  "lastManaPaymentPerf",
  "lastSnapshotPerf",
  "lastWorkCounters",
  "getSemanticThreshold",
  "injectTranscriptRandomSeeds",
  "loadDecks",
  "loadDemoDecks",
  "objectDetails",
  "inspectorActions",
  "previewCustomCard",
  "previewCastTargets",
  "previewCryptoRequirements",
  "registrySize",
  "reset",
  "resetEmpty",
  "sampleLoadedDeckSeed",
  "setLife",
  "setAutoCleanupDiscard",
  "setSemanticThreshold",
  "setPerspective",
  "snapshot",
  "snapshotJson",
  "startMatch",
  "switchPerspective",
  "uiState",
  "validateMatchConfig",
  "revealHiddenObject",
  "revealHiddenPosition",
  "revealHiddenPositions",
  "revealHiddenSlot",
  "ziffleBuildRevealToken",
  "ziffleBuildRevealTokens",
  "ziffleBuildShuffleStep",
  "ziffleKeygen",
  "ziffleRevealCard",
  "ziffleRevealCards",
  "ziffleVerifyShuffle",
];

const ZIFFLE_WORKER_METHODS = new Set([
  "ziffleBuildRevealToken",
  "ziffleBuildRevealTokens",
  "ziffleBuildShuffleStep",
  "ziffleKeygen",
  "ziffleRevealCard",
  "ziffleRevealCards",
  "ziffleVerifyShuffle",
]);

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function toError(raw) {
  if (raw instanceof Error) return raw;
  if (typeof raw === "string") return new Error(raw);
  if (raw && typeof raw === "object") {
    const err = new Error(raw.message || "Unknown worker error");
    if (raw.stack) err.stack = raw.stack;
    err.name = raw.name || err.name;
    return err;
  }
  return new Error("Unknown worker error");
}

function preferredZiffleWorkerCount() {
  const configured = Number(import.meta.env?.VITE_ZIFFLE_WORKER_POOL_SIZE);
  if (Number.isFinite(configured) && configured > 0) {
    return Math.max(1, Math.min(8, Math.floor(configured)));
  }
  const cores = Number(globalThis.navigator?.hardwareConcurrency || 2);
  if (!Number.isFinite(cores) || cores <= 2) return 1;
  if (cores <= 4) return 2;
  if (cores <= 6) return 3;
  return 4;
}

function createGameProxy(callWorker, callZiffleWorker) {
  const proxy = {};
  for (const method of WORKER_METHODS) {
    proxy[method] = (...args) => {
      if (ZIFFLE_WORKER_METHODS.has(method) && typeof callZiffleWorker === "function") {
        return callZiffleWorker(method, args);
      }
      return callWorker(method, args);
    };
  }
  return proxy;
}

function resolveAssetBaseUrl() {
  const configuredBase = import.meta.env.BASE_URL || "/";
  if (configuredBase !== "./") {
    return new URL(configuredBase, window.location.href).href;
  }

  const current = new URL(window.location.href);
  if (!current.pathname.endsWith("/") && !/\.[^/]+$/.test(current.pathname)) {
    current.pathname = `${current.pathname}/`;
  }
  current.search = "";
  current.hash = "";
  return new URL("./", current.href).href;
}

export function useWasmGame() {
  const [game, setGame] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [progress, setProgress] = useState(0);
  const [phase, setPhase] = useState("module");
  const [registryCount, setRegistryCount] = useState(0);
  const [registryTotal, setRegistryTotal] = useState(0);
  const initialized = useRef(false);

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;

    let disposed = false;
    let nextRequestId = 1;
    let initStartedAt = 0;
    const pending = new Map();
    const watchdogTimeoutMs = engineWorkerTimeoutMs(
      import.meta.env?.VITE_ENGINE_WORKER_TIMEOUT_MS
    );
    const workerAssetVersion = `${Date.now()}-${Math.floor(Math.random() * 1e9)}`;
    const assetBaseUrl = resolveAssetBaseUrl();
    let worker = null;
    let workerGeneration = 0;
    let workerReady = null;
    let resolveWorkerReady = null;
    let rejectWorkerReady = null;
    let recoveryPromise = null;
    let lastSyncCheckpoint = null;
    let lastPerspective = 0;
    let nextZiffleRequestId = 1;
    let zifflePool = [];
    let zifflePoolReady = null;
    let ziffleRoundRobin = 0;
    const zifflePending = new Map();

    const rejectPending = (err, generation = null) => {
      for (const [id, request] of pending) {
        if (generation !== null && request.generation !== generation) continue;
        pending.delete(id);
        if (request.timeoutId !== null) clearTimeout(request.timeoutId);
        endEngineRequest(id);
        request.reject(err);
      }
    };

    const rejectZifflePending = (err) => {
      for (const { reject } of zifflePending.values()) reject(err);
      zifflePending.clear();
    };

    const rejectZiffleWorkerPending = (workerEntry, err) => {
      for (const [id, pendingRequest] of zifflePending.entries()) {
        if (pendingRequest.workerEntry !== workerEntry) continue;
        zifflePending.delete(id);
        pendingRequest.reject(err);
      }
      if (workerEntry) workerEntry.pending = 0;
    };

    const postWorkerCall = (method, args = [], { watchdog = true } = {}) =>
      new Promise((resolve, reject) => {
        if (disposed) {
          reject(new Error("WASM worker is not available"));
          return;
        }
        if (!worker) {
          reject(new Error("WASM worker is restarting"));
          return;
        }
        const id = nextRequestId++;
        const generation = workerGeneration;
        const timeoutId = watchdog && shouldWatchEngineMethod(method)
          ? setTimeout(() => {
              if (!pending.has(id) || generation !== workerGeneration) return;
              void recoverWorkerFromStall(method, generation);
            }, watchdogTimeoutMs)
          : null;
        pending.set(id, { resolve, reject, method, args, generation, timeoutId });
        beginEngineRequest(id, method);
        try { worker.postMessage({ type: "call", id, method, args }); }
        catch (error) {
          pending.delete(id);
          if (timeoutId !== null) clearTimeout(timeoutId);
          endEngineRequest(id);
          reject(error);
        }
      });

    const callWorker = async (method, args = []) => {
      if (recoveryPromise) await recoveryPromise;
      if (workerReady) await workerReady;
      return postWorkerCall(method, args);
    };

    const selectZiffleWorker = () => {
      let best = null;
      for (const entry of zifflePool) {
        if (!best || entry.pending < best.pending) best = entry;
      }
      if (best) return best;
      const fallback = zifflePool[ziffleRoundRobin % Math.max(1, zifflePool.length)] || null;
      ziffleRoundRobin += 1;
      return fallback;
    };

    const ensureZifflePool = () => {
      if (zifflePoolReady) return zifflePoolReady;
      const size = preferredZiffleWorkerCount();
      zifflePoolReady = new Promise((resolve, reject) => {
        let settled = false;
        let readyCount = 0;
        const fail = (err) => {
          rejectZifflePending(err);
          if (settled) return;
          settled = true;
          reject(err);
        };
        zifflePool = Array.from({ length: size }, (_, workerIndex) => {
          const ziffleWorker = new Worker(
            new URL("../workers/ziffleWorker.js", import.meta.url),
            { type: "module" }
          );
          const entry = {
            index: workerIndex,
            worker: ziffleWorker,
            pending: 0,
            ready: false,
          };
          ziffleWorker.addEventListener("message", (event) => {
            if (disposed) return;
            const msg = event.data || {};
            if (msg.type === "ready") {
              if (!entry.ready) {
                entry.ready = true;
                readyCount += 1;
              }
              if (!settled && readyCount === size) {
                settled = true;
                resolve(zifflePool);
              }
              return;
            }
            if (msg.type === "result") {
              const req = zifflePending.get(msg.id);
              if (!req) return;
              zifflePending.delete(msg.id);
              req.workerEntry.pending = Math.max(0, req.workerEntry.pending - 1);
              if (msg.ok) req.resolve(msg.result);
              else req.reject(toError(msg.error));
              return;
            }
            if (msg.type === "error") {
              const err = toError(msg.error);
              rejectZiffleWorkerPending(entry, err);
              fail(err);
            }
          });
          ziffleWorker.addEventListener("error", (event) => {
            const err = new Error(event.message || "Ziffle worker crashed");
            rejectZiffleWorkerPending(entry, err);
            fail(err);
          });
          ziffleWorker.postMessage({ type: "init", workerIndex });
          return entry;
        });
      });
      return zifflePoolReady;
    };

    const callZiffleWorker = async (method, args = []) => {
      if (disposed) throw new Error("WASM worker is not available");
      await ensureZifflePool();
      if (disposed) throw new Error("WASM worker is not available");
      return new Promise((resolve, reject) => {
        const workerEntry = selectZiffleWorker();
        if (!workerEntry?.worker) {
          reject(new Error("Ziffle worker pool is not available"));
          return;
        }
        const id = nextZiffleRequestId++;
        workerEntry.pending += 1;
        zifflePending.set(id, { resolve, reject, workerEntry });
        workerEntry.worker.postMessage({ type: "call", id, method, args });
      });
    };

    const gameProxy = createGameProxy(callWorker, callZiffleWorker);
    const priorityAnalysisListeners = new Set();
    let latestPriorityAnalysis = null;
    gameProxy.latestPriorityAnalysis = () => latestPriorityAnalysis;
    gameProxy.subscribePriorityAnalysis = (listener) => {
      priorityAnalysisListeners.add(listener);
      return () => priorityAnalysisListeners.delete(listener);
    };

    const finishReady = async () => {
      const elapsed = initStartedAt > 0 ? performance.now() - initStartedAt : MIN_INIT_PHASE_MS;
      const remaining = Math.max(0, MIN_INIT_PHASE_MS - elapsed);
      if (remaining > 0) await sleep(remaining);
      if (disposed) return;
      setProgress(1);
      setGame(gameProxy);
      setLoading(false);
    };

    let initialReady = false;

    const detachWorker = (target) => {
      if (!target) return;
      if (target.__ironsmithMessageHandler) {
        target.removeEventListener("message", target.__ironsmithMessageHandler);
      }
      if (target.__ironsmithErrorHandler) {
        target.removeEventListener("error", target.__ironsmithErrorHandler);
      }
    };

    const onMessage = (event, generation, target) => {
      if (disposed || target !== worker || generation !== workerGeneration) return;
      const msg = event.data || {};

      if (msg.type === "progress") {
        if (!initialReady && typeof msg.phase === "string") {
          setPhase(msg.phase);
          if (msg.phase === "init" && initStartedAt === 0) {
            initStartedAt = performance.now();
          }
        }
        if (!initialReady && typeof msg.progress === "number") {
          const clamped = Math.max(0, Math.min(1, msg.progress));
          setProgress(clamped);
        }
        if (typeof msg.registryCount === "number") {
          setRegistryCount(Math.max(0, Math.floor(msg.registryCount)));
        }
        if (typeof msg.registryTotal === "number") {
          setRegistryTotal(Math.max(0, Math.floor(msg.registryTotal)));
        }
        return;
      }

      if (msg.type === "registry") {
        if (typeof msg.loaded === "number") {
          setRegistryCount(Math.max(0, Math.floor(msg.loaded)));
        }
        if (typeof msg.total === "number") {
          setRegistryTotal(Math.max(0, Math.floor(msg.total)));
        }
        return;
      }

      if (msg.type === "priorityAnalysis") {
        latestPriorityAnalysis = msg;
        for (const listener of priorityAnalysisListeners) listener(msg);
        return;
      }
      if (msg.type === "priorityAnalysisError") {
        console.error("Priority analysis failed; explicit passing remains available", msg.error);
        return;
      }
      if (msg.type === "result") {
        const req = pending.get(msg.id);
        if (!req || req.generation !== generation) return;
        pending.delete(msg.id);
        if (req.timeoutId !== null) clearTimeout(req.timeoutId);
        endEngineRequest(msg.id);
        if (msg.ok) {
          if (req.method === "exportSyncCheckpoint" && msg.result) {
            lastSyncCheckpoint = msg.result;
            const checkpointPerspective = Number(msg.result?.perspective);
            if (Number.isInteger(checkpointPerspective) && checkpointPerspective >= 0) {
              lastPerspective = checkpointPerspective;
            }
          } else if (req.method === "importSyncCheckpoint" && req.args[0]) {
            lastSyncCheckpoint = req.args[0];
            const importedPerspective = Number(req.args[1]);
            if (Number.isInteger(importedPerspective) && importedPerspective >= 0) {
              lastPerspective = importedPerspective;
            }
          } else if (req.method === "setPerspective") {
            const nextPerspective = Number(req.args[0]);
            if (Number.isInteger(nextPerspective) && nextPerspective >= 0) {
              lastPerspective = nextPerspective;
            }
          }
          req.resolve(msg.result);
        } else {
          req.reject(toError(msg.error));
        }
        return;
      }

      if (msg.type === "ready") {
        resolveWorkerReady?.();
        resolveWorkerReady = null;
        rejectWorkerReady = null;
        if (!initialReady) {
          initialReady = true;
          finishReady().catch((err) => {
            if (!disposed) {
              setError(toError(err));
              setLoading(false);
            }
          });
        }
        return;
      }

      if (msg.type === "error") {
        const err = toError(msg.error);
        rejectWorkerReady?.(err);
        rejectWorkerReady = null;
        resolveWorkerReady = null;
        if (!initialReady) {
          rejectPending(err, generation);
          setError(err);
          setLoading(false);
        } else {
          void recoverWorkerAfterFailure(err, generation).catch(() => {});
        }
      }
    };

    const onWorkerError = (event, generation, target) => {
      if (disposed || target !== worker || generation !== workerGeneration) return;
      const err = new Error(event.message || "WASM worker crashed");
      rejectWorkerReady?.(err);
      rejectWorkerReady = null;
      resolveWorkerReady = null;
      if (!initialReady) {
        rejectPending(err, generation);
        setError(err);
        setLoading(false);
      } else {
        void recoverWorkerAfterFailure(err, generation).catch(() => {});
      }
    };

    const startEngineWorker = (recovery = false) => {
      const generation = workerGeneration + 1;
      workerGeneration = generation;
      const nextWorker = new Worker(
        new URL("../workers/wasmGameWorker.js", import.meta.url),
        { type: "module" }
      );
      worker = nextWorker;
      workerReady = new Promise((resolve, reject) => {
        resolveWorkerReady = resolve;
        rejectWorkerReady = reject;
      });
      // Initialization failures are also surfaced through React state. Attach a
      // handler immediately so a failed first boot cannot become an unhandled
      // rejection before a caller has a chance to await workerReady.
      void workerReady.catch(() => {});
      const messageHandler = (event) => onMessage(event, generation, nextWorker);
      const errorHandler = (event) => onWorkerError(event, generation, nextWorker);
      nextWorker.__ironsmithMessageHandler = messageHandler;
      nextWorker.__ironsmithErrorHandler = errorHandler;
      nextWorker.addEventListener("message", messageHandler);
      nextWorker.addEventListener("error", errorHandler);
      nextWorker.postMessage({
        type: "init",
        assetBaseUrl,
        assetVersion: workerAssetVersion,
        recovery,
      });
      return nextWorker;
    };

    const replaceEngineWorker = async (failure, generation) => {
      if (disposed || generation !== workerGeneration) return false;
      const failedWorker = worker;
      rejectPending(failure, generation);
      detachWorker(failedWorker);
      failedWorker?.terminate();
      worker = null;

      startEngineWorker(true);
      await workerReady;
      if (lastSyncCheckpoint) {
        await postWorkerCall(
          "importSyncCheckpoint",
          [lastSyncCheckpoint, lastPerspective],
          { watchdog: false }
        );
        recordDiagnosticEvent("engine:worker_recovered", {
          generation: workerGeneration,
          restoredCheckpoint: true,
        });
        return true;
      }
      recordDiagnosticEvent("engine:worker_recovered", {
        generation: workerGeneration,
        restoredCheckpoint: false,
      });
      return false;
    };

    const recoverWorkerAfterFailure = (failure, generation) => {
      if (recoveryPromise) return recoveryPromise;
      recoveryPromise = replaceEngineWorker(failure, generation)
        .catch((recoveryError) => {
          const err = toError(recoveryError);
          recordDiagnosticEvent("engine:worker_recovery_failed", {
            generation: workerGeneration,
            message: err.message,
          });
          if (!disposed) setError(err);
          throw err;
        })
        .finally(() => {
          recoveryPromise = null;
        });
      return recoveryPromise;
    };

    const recoverWorkerFromStall = (method, generation) => {
      const failure = makeEngineWorkerStallError(
        method,
        watchdogTimeoutMs,
        Boolean(lastSyncCheckpoint)
      );
      console.warn(failure.message, {
        generation,
        checkpointAvailable: Boolean(lastSyncCheckpoint),
      });
      recordDiagnosticEvent("engine:worker_stall", {
        method,
        timeoutMs: watchdogTimeoutMs,
        generation,
        checkpointAvailable: Boolean(lastSyncCheckpoint),
      });
      return recoverWorkerAfterFailure(failure, generation);
    };

    setLoading(true);
    setError(null);
    setGame(null);
    setProgress(0);
    setPhase("module");
    setRegistryCount(0);
    setRegistryTotal(0);

    startEngineWorker(false);

    return () => {
      disposed = true;
      detachWorker(worker);
      worker?.terminate();
      worker = null;
      rejectPending(new Error("WASM worker terminated"));
      for (const entry of zifflePool) {
        entry.worker?.terminate();
      }
      zifflePool = [];
      rejectZifflePending(new Error("Ziffle worker pool terminated"));
    };
  }, []);

  return { game, loading, error, progress, phase, registryCount, registryTotal };
}
