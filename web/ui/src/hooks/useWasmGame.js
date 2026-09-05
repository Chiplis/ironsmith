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
  "getSemanticThreshold",
  "injectTranscriptRandomSeeds",
  "loadDecks",
  "loadDemoDecks",
  "objectDetails",
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
    let nextZiffleRequestId = 1;
    let zifflePool = [];
    let zifflePoolReady = null;
    let ziffleRoundRobin = 0;
    const zifflePending = new Map();

    const worker = new Worker(
      new URL("../workers/wasmGameWorker.js", import.meta.url),
      { type: "module" }
    );

    const rejectPending = (err) => {
      for (const { reject } of pending.values()) reject(err);
      pending.clear();
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

    const callWorker = (method, args = []) =>
      new Promise((resolve, reject) => {
        if (disposed) {
          reject(new Error("WASM worker is not available"));
          return;
        }
        const id = nextRequestId++;
        pending.set(id, { resolve, reject });
        worker.postMessage({ type: "call", id, method, args });
      });

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

    const finishReady = async () => {
      const elapsed = initStartedAt > 0 ? performance.now() - initStartedAt : MIN_INIT_PHASE_MS;
      const remaining = Math.max(0, MIN_INIT_PHASE_MS - elapsed);
      if (remaining > 0) await sleep(remaining);
      if (disposed) return;
      setProgress(1);
      setGame(gameProxy);
      setLoading(false);
    };

    const onMessage = (event) => {
      if (disposed) return;
      const msg = event.data || {};

      if (msg.type === "progress") {
        if (typeof msg.phase === "string") {
          setPhase(msg.phase);
          if (msg.phase === "init" && initStartedAt === 0) {
            initStartedAt = performance.now();
          }
        }
        if (typeof msg.progress === "number") {
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

      if (msg.type === "result") {
        const req = pending.get(msg.id);
        if (!req) return;
        pending.delete(msg.id);
        if (msg.ok) req.resolve(msg.result);
        else req.reject(toError(msg.error));
        return;
      }

      if (msg.type === "ready") {
        finishReady().catch((err) => {
          if (!disposed) {
            setError(toError(err));
            setLoading(false);
          }
        });
        return;
      }

      if (msg.type === "error") {
        const err = toError(msg.error);
        rejectPending(err);
        setError(err);
        setLoading(false);
      }
    };

    const onWorkerError = (event) => {
      if (disposed) return;
      const err = new Error(event.message || "WASM worker crashed");
      rejectPending(err);
      setError(err);
      setLoading(false);
    };

    worker.addEventListener("message", onMessage);
    worker.addEventListener("error", onWorkerError);

    setLoading(true);
    setError(null);
    setGame(null);
    setProgress(0);
    setPhase("module");
    setRegistryCount(0);
    setRegistryTotal(0);

    const assetBaseUrl = resolveAssetBaseUrl();
    worker.postMessage({ type: "init", assetBaseUrl });

    return () => {
      disposed = true;
      worker.removeEventListener("message", onMessage);
      worker.removeEventListener("error", onWorkerError);
      worker.terminate();
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
