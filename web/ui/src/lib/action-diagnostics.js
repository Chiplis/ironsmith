// Action latency diagnostics.
//
// A player who clicks "pass priority" and sees nothing for twenty seconds
// cannot tell whether the browser is jammed, the engine is busy, a peer is
// slow to answer, or the connection is dead. This store keeps a short history
// of action traces (one per local action, with timestamped stages from click
// to paint), peer transport health (heartbeat round trips, last message seen,
// bytes moved) and main-thread health (event-loop stalls), so the diagnostics
// sheet and `window.__ironsmithDiagnostics` can say which leg was slow.
//
// Everything here is best-effort bookkeeping: it must never throw into the
// game path, and it must stay cheap when nobody is looking.

const MAX_TRACES = 40;
const MAX_EVENTS = 240;
const MAX_STALLS = 60;
const STALL_WINDOW_MS = 60_000;
const STALL_THRESHOLD_MS = 50;
const HEARTBEAT_INTERVAL_MS = 500;
const RTT_SAMPLES = 20;

const now = () => (globalThis.performance?.now?.() ?? Date.now());

const store = {
  traces: [],
  events: [],
  peers: new Map(),
  stalls: [],
  mainThread: { lagMs: 0, worstStallMs: 0, worstStallAt: null, lastTickAt: null, longTasks: 0 },
  engine: null,
  listeners: new Set(),
  version: 0,
  traceSequence: 0,
};

function notify() {
  store.version += 1;
  for (const listener of store.listeners) {
    try { listener(store.version); } catch { /* listeners must not break bookkeeping */ }
  }
}

function compact(value, depth = 0) {
  if (value == null || typeof value !== "object") return value;
  if (depth > 2) return Array.isArray(value) ? `[${value.length}]` : "{…}";
  if (Array.isArray(value)) return value.slice(0, 12).map((item) => compact(item, depth + 1));
  const out = {};
  for (const [key, item] of Object.entries(value).slice(0, 24)) out[key] = compact(item, depth + 1);
  return out;
}

// ---------------------------------------------------------------------------
// Action traces

export function beginActionTrace({ label, command = null, requestId = null, actionIntentKey = null, mode = "local" } = {}) {
  for (const open of openTraces()) {
    open.done = true;
    open.outcome = "superseded";
    open.totalMs = now() - open.startedAt;
  }
  store.traceSequence += 1;
  const trace = {
    id: `trace-${store.traceSequence}`,
    label: String(label || command?.type || "action"),
    command: compact(command),
    requestId: requestId == null ? null : String(requestId),
    actionIntentKey: actionIntentKey == null ? null : String(actionIntentKey),
    mode,
    startedAt: now(),
    startedAtWall: Date.now(),
    stages: [],
    done: false,
    outcome: null,
    totalMs: null,
  };
  store.traces.push(trace);
  if (store.traces.length > MAX_TRACES) store.traces.splice(0, store.traces.length - MAX_TRACES);
  notify();
  return trace.id;
}

// Keyed lookups (request id, intent key) may address a finished trace: late
// echoes still belong to the action that caused them. Unkeyed or unknown refs
// fall back to the newest open trace, which is the action the player is
// waiting on right now.
function findTrace(ref) {
  if (!ref) return openTraces().at(-1) || null;
  if (typeof ref === "string") {
    return store.traces.find((trace) => trace.id === ref)
      || store.traces.findLast((trace) => trace.requestId === ref || trace.actionIntentKey === ref)
      || openTraces().at(-1)
      || null;
  }
  const { id, requestId, actionIntentKey } = ref;
  if (id) return store.traces.find((trace) => trace.id === id) || null;
  return store.traces.findLast((trace) => (
    (requestId != null && trace.requestId === String(requestId))
    || (actionIntentKey != null && trace.actionIntentKey === String(actionIntentKey))
  )) || openTraces().at(-1) || null;
}

function openTraces() {
  return store.traces.filter((trace) => !trace.done);
}

// Attach identifiers the pipeline only learns later (request ids, intent keys)
// so subsequent stages can find the trace by them.
export function tagActionTrace(ref, { requestId, actionIntentKey } = {}) {
  const trace = findTrace(ref);
  if (!trace) return null;
  if (requestId != null) trace.requestId = String(requestId);
  if (actionIntentKey != null) trace.actionIntentKey = String(actionIntentKey);
  return trace.id;
}

export function markActionStage(ref, name, meta = null) {
  const trace = findTrace(ref);
  if (!trace) return null;
  const at = now();
  const previous = trace.stages.at(-1);
  trace.stages.push({
    name: String(name),
    at,
    sinceStartMs: at - trace.startedAt,
    sincePreviousMs: at - (previous ? previous.at : trace.startedAt),
    meta: compact(meta),
  });
  notify();
  return trace.id;
}

export function completeActionTrace(ref, { outcome = "ok", meta = null } = {}) {
  const trace = findTrace(ref);
  if (!trace || trace.done) return null;
  markActionStage(trace.id, outcome === "ok" ? "done" : outcome, meta);
  trace.done = true;
  trace.outcome = outcome;
  trace.totalMs = now() - trace.startedAt;
  notify();
  return trace.id;
}

export function currentActionTrace() {
  return openTraces().at(-1) || null;
}

// ---------------------------------------------------------------------------
// Events and peers

export function recordDiagnosticEvent(kind, meta = null) {
  store.events.push({ kind: String(kind), at: now(), atWall: Date.now(), meta: compact(meta) });
  if (store.events.length > MAX_EVENTS) store.events.splice(0, store.events.length - MAX_EVENTS);
  notify();
}

function peerEntry(peerId, name) {
  const key = String(peerId || "unknown");
  let entry = store.peers.get(key);
  if (!entry) {
    entry = { peerId: key, name: "", rttMs: null, rttSamples: [], lastReceivedAt: null, lastSentAt: null, received: 0, sent: 0, bytesIn: 0, bytesOut: 0, lastType: "", state: "unknown" };
    store.peers.set(key, entry);
  }
  if (name) entry.name = String(name);
  return entry;
}

export function recordPeerRtt(peerId, rttMs, name) {
  if (!Number.isFinite(rttMs) || rttMs < 0) return;
  const entry = peerEntry(peerId, name);
  entry.rttMs = rttMs;
  entry.rttSamples.push(rttMs);
  if (entry.rttSamples.length > RTT_SAMPLES) entry.rttSamples.shift();
  notify();
}

export function recordPeerMessage(peerId, direction, type, bytes = 0, name) {
  const entry = peerEntry(peerId, name);
  const at = now();
  if (direction === "in") {
    entry.lastReceivedAt = at;
    entry.received += 1;
    entry.bytesIn += Number(bytes) || 0;
    entry.lastType = String(type || "");
  } else {
    entry.lastSentAt = at;
    entry.sent += 1;
    entry.bytesOut += Number(bytes) || 0;
  }
  // Heartbeats are noise at the event level; every other message is worth a line.
  if (!/^peer_heartbeat/.test(String(type || ""))) {
    recordDiagnosticEvent(direction === "in" ? "message:in" : "message:out", { peer: entry.name || entry.peerId, type, bytes });
  } else {
    notify();
  }
}

export function recordPeerState(peerId, state, name) {
  const entry = peerEntry(peerId, name);
  if (entry.state !== state) {
    entry.state = String(state);
    recordDiagnosticEvent("peer:state", { peer: entry.name || entry.peerId, state });
  }
}

export function forgetPeer(peerId) {
  if (store.peers.delete(String(peerId))) notify();
}

export function recordEnginePerf(perf) {
  if (!perf) return;
  store.engine = {
    at: now(),
    queueWaitMs: Number(perf.queueWaitMs ?? perf.queue_wait_ms ?? 0),
    wasmCallMs: Number(perf.wasmCallMs ?? perf.wasm_call_ms ?? 0),
    totalWorkerMs: Number(perf.totalWorkerMs ?? perf.total_worker_ms ?? 0),
    method: String(perf.method || ""),
  };
  notify();
}

// ---------------------------------------------------------------------------
// Main-thread health

let monitorHandle = null;
export function startMainThreadMonitor() {
  if (monitorHandle || typeof window === "undefined") return () => {};
  let expected = now() + HEARTBEAT_INTERVAL_MS;
  const tick = () => {
    const at = now();
    const lag = Math.max(0, at - expected);
    store.mainThread.lagMs = lag;
    store.mainThread.lastTickAt = at;
    if (lag >= STALL_THRESHOLD_MS) {
      store.stalls.push({ at, durationMs: lag, source: "timer" });
      if (store.stalls.length > MAX_STALLS) store.stalls.shift();
    }
    const cutoff = at - STALL_WINDOW_MS;
    const recent = store.stalls.filter((stall) => stall.at >= cutoff);
    const worst = recent.reduce((best, stall) => (stall.durationMs > (best?.durationMs || 0) ? stall : best), null);
    store.mainThread.worstStallMs = worst?.durationMs || 0;
    store.mainThread.worstStallAt = worst?.at ?? null;
    expected = at + HEARTBEAT_INTERVAL_MS;
    monitorHandle.timer = window.setTimeout(tick, HEARTBEAT_INTERVAL_MS);
    notify();
  };
  monitorHandle = { timer: window.setTimeout(tick, HEARTBEAT_INTERVAL_MS), observer: null };
  if (typeof PerformanceObserver === "function") {
    try {
      const observer = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          store.mainThread.longTasks += 1;
          store.stalls.push({ at: entry.startTime, durationMs: entry.duration, source: "longtask" });
        }
        if (store.stalls.length > MAX_STALLS) store.stalls.splice(0, store.stalls.length - MAX_STALLS);
      });
      observer.observe({ type: "longtask", buffered: true });
      monitorHandle.observer = observer;
    } catch { /* longtask timing is optional */ }
  }
  return stopMainThreadMonitor;
}

export function stopMainThreadMonitor() {
  if (!monitorHandle) return;
  window.clearTimeout(monitorHandle.timer);
  monitorHandle.observer?.disconnect();
  monitorHandle = null;
}

// ---------------------------------------------------------------------------
// Reading

export function subscribeDiagnostics(listener) {
  store.listeners.add(listener);
  return () => store.listeners.delete(listener);
}

export function diagnosticsVersion() {
  return store.version;
}

export function getDiagnosticsSnapshot() {
  const at = now();
  return {
    at,
    atWall: Date.now(),
    traces: store.traces.slice().reverse(),
    current: currentActionTrace(),
    events: store.events.slice().reverse(),
    peers: [...store.peers.values()].map((peer) => ({
      ...peer,
      rttAvgMs: peer.rttSamples.length ? peer.rttSamples.reduce((sum, value) => sum + value, 0) / peer.rttSamples.length : null,
      sinceReceivedMs: peer.lastReceivedAt == null ? null : at - peer.lastReceivedAt,
      sinceSentMs: peer.lastSentAt == null ? null : at - peer.lastSentAt,
    })),
    mainThread: { ...store.mainThread, stalls: store.stalls.filter((stall) => stall.at >= at - STALL_WINDOW_MS) },
    engine: store.engine,
  };
}

export function exportDiagnostics(extra = null) {
  return {
    exportedAt: new Date().toISOString(),
    ...getDiagnosticsSnapshot(),
    perfEvents: typeof window !== "undefined" && Array.isArray(window.__ironsmithPerfEvents) ? window.__ironsmithPerfEvents.slice(-100) : [],
    extra: compact(extra),
  };
}

export function resetDiagnostics() {
  store.traces = [];
  store.events = [];
  store.stalls = [];
  store.mainThread = { lagMs: 0, worstStallMs: 0, worstStallAt: null, lastTickAt: null, longTasks: 0 };
  store.engine = null;
  notify();
}

if (typeof window !== "undefined") {
  window.__ironsmithDiagnostics = { snapshot: getDiagnosticsSnapshot, export: exportDiagnostics, reset: resetDiagnostics };
}
