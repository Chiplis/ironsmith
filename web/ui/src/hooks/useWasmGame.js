import { useEffect, useRef, useState } from "react";

const MIN_INIT_PHASE_MS = 180;

const WORKER_METHODS = [
  "addCardToHand",
  "addCardToZone",
  "addLifeDelta",
  "advancePhase",
  "cancelDecision",
  "cardsMeetingThreshold",
  "dispatch",
  "drawCard",
  "drawOpeningHands",
  "getCardSemanticScore",
  "getSemanticThreshold",
  "loadDecks",
  "loadDemoDecks",
  "objectDetails",
  "registrySize",
  "reset",
  "setLife",
  "setAutoCleanupDiscard",
  "setSemanticThreshold",
  "setPerspective",
  "snapshot",
  "snapshotJson",
  "startMatch",
  "switchPerspective",
  "uiState",
];

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function buildModuleCandidates() {
  const pathname = window.location.pathname;
  const baseDir = (() => {
    if (pathname.endsWith("/")) return pathname;
    if (pathname.includes(".")) return pathname.slice(0, pathname.lastIndexOf("/") + 1);
    return `${pathname}/`;
  })();

  return [
    `${window.location.origin}${baseDir}pkg/ironsmith.js`,
    `${window.location.origin}${baseDir}pkg/ironsmith.js`,
    `${window.location.origin}${baseDir}pkg/pkg/ironsmith.js`,
    `${window.location.origin}${baseDir}pkg/pkg/ironsmith.js`,
    "./pkg/ironsmith.js",
    "./pkg/ironsmith.js",
    "./pkg/pkg/ironsmith.js",
    "./pkg/pkg/ironsmith.js",
    "../../pkg/ironsmith.js",
    "../../pkg/ironsmith.js",
  ];
}

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

function createGameProxy(callWorker) {
  const proxy = {};
  for (const method of WORKER_METHODS) {
    proxy[method] = (...args) => callWorker(method, args);
  }
  return proxy;
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

    const worker = new Worker(
      new URL("../workers/wasmGameWorker.js", import.meta.url),
      { type: "module" }
    );

    const rejectPending = (err) => {
      for (const { reject } of pending.values()) reject(err);
      pending.clear();
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

    const gameProxy = createGameProxy(callWorker);

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

    worker.postMessage({
      type: "init",
      pageHref: window.location.href,
      candidates: buildModuleCandidates(),
    });

    return () => {
      disposed = true;
      worker.removeEventListener("message", onMessage);
      worker.removeEventListener("error", onWorkerError);
      worker.terminate();
      rejectPending(new Error("WASM worker terminated"));
    };
  }, []);

  return { game, loading, error, progress, phase, registryCount, registryTotal };
}
