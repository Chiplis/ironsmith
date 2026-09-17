// Engine journal: the replayable half of the diagnostics bundle.
//
// The latency traces in action-diagnostics.js say *which leg* was slow. They
// cannot say why, because nobody can re-run the slow call: a diagnostics export
// carries the last published UI snapshot, which is a redacted view (opponent
// hands are counts, libraries are counts) rather than engine state. A 120 s
// dispatch reported that way is unactionable.
//
// This journal records every mutating worker call in order — the same calls the
// worker already serializes — so the session can be reconstructed from a fresh
// engine by issuing them again. `startMatch` carries the decks and seed,
// `addCardToZone` carries the sandbox setup, `dispatch` carries the commands.
// Replaying that prefix reproduces the board that made the call slow.
//
// Two things keep the journal honest rather than merely plausible:
//
//   * Every entry records a fingerprint of the state the call produced. A
//     replay that diverges is detected at the entry where it diverged, instead
//     of silently profiling a different game.
//   * Overflow truncates the *tail*, never the middle. A prefix replays; a
//     journal with a hole in it is worthless and must not look usable.
//
// Like the rest of the diagnostics bookkeeping this must never throw into the
// game path and must stay cheap when nobody is looking.

import { isGameRead } from "./game-methods.js";

export const JOURNAL_VERSION = 1;

const MAX_ENTRIES = 5000;
const MAX_TOTAL_ARG_BYTES = 16 * 1024 * 1024;
const MAX_ENTRY_ARG_BYTES = 4 * 1024 * 1024;

const now = () => (globalThis.performance?.now?.() ?? Date.now());

// Reads are skipped outright: they cannot move the engine, so replaying them
// would only slow the harness down. `isGameRead` is the same predicate the
// worker proxy uses to decide whether a call invalidates the snapshot version,
// so the journal and the snapshot bookkeeping can never disagree about what a
// mutation is.
const isMutation = (method) => !isGameRead(method);

// Methods whose arguments are pure transport plumbing for a peer session. They
// are recorded by name only: replaying a match from its transcript is the
// audit-replay path's job, not this one.
const ARGS_NEVER_CAPTURED = new Set([
  "applyVerifiedHiddenLibraryShuffle",
  "importSyncCheckpoint",
  "injectTranscriptRandomSeeds",
  "replayTrustedActions",
  "replayTrustedMatch",
]);

const store = {
  entries: [],
  sequence: 0,
  startedAtMs: null,
  startedAtWall: null,
  argBytes: 0,
  overflowed: false,
  overflowAtSeq: null,
  // "full" keeps arguments, which include card identities. Multiplayer holds
  // other people's hidden information, so the default there is "redacted" and
  // the operator has to opt in.
  policy: "full",
  workerInit: null,
  cardRoutes: null,
};

function safeClone(value) {
  if (value === undefined) return null;
  try {
    return JSON.parse(JSON.stringify(value));
  } catch {
    return null;
  }
}

// Serialize once and measure the text, rather than cloning and then stringifying
// the clone to size it. This runs on every engine mutation, including each
// opponent auto-pass inside a settle loop.
function cloneWithSize(value) {
  let text;
  try {
    text = JSON.stringify(value);
  } catch {
    return { clone: null, bytes: 0 };
  }
  if (text === undefined) return { clone: null, bytes: 0 };
  return { clone: JSON.parse(text), bytes: text.length };
}

// Enough shape to tell "the deck list was there" from "the deck list was empty"
// without carrying the cards themselves. The argument list keeps its positions:
// which argument was a config and which was a card name is most of what a
// redacted entry can still say.
function argListShape(args) {
  return Array.isArray(args) ? args.map((arg) => argShape(arg)) : argShape(args);
}

function argShape(value, depth = 0) {
  if (value == null) return value === undefined ? "undefined" : "null";
  if (Array.isArray(value)) {
    return depth >= 2 ? `array(${value.length})` : { array: value.length, of: argShape(value[0], depth + 1) };
  }
  if (typeof value === "object") {
    if (depth >= 2) return "object";
    return Object.fromEntries(
      Object.keys(value).slice(0, 24).map((key) => [key, argShape(value[key], depth + 1)])
    );
  }
  if (typeof value === "string") return `string(${value.length})`;
  return typeof value;
}

// A cheap, comparable description of where the engine landed. The replay
// harness diffs this per entry; anything that would change how long the next
// call takes should be visible here.
function stateFingerprint(state) {
  if (!state || typeof state !== "object") return null;
  const decision = state.decision || null;
  return {
    snapshotId: state.snapshot_id ?? null,
    turn: state.turn_number ?? null,
    phase: state.phase ?? null,
    step: state.step ?? null,
    activePlayer: state.active_player ?? null,
    priorityPlayer: state.priority_player ?? null,
    stackSize: state.stack_size ?? null,
    battlefieldSize: state.battlefield_size ?? null,
    exileSize: state.exile_size ?? null,
    decisionKind: decision?.kind ?? null,
    decisionPlayer: decision?.player ?? null,
    // Whether the menu was the deferred exact one or the provisional one
    // changes what the *next* dispatch has to do, so a replay that disagrees
    // here is not reproducing the same session.
    analysisComplete: decision?.analysis_complete ?? null,
    actionCount: Array.isArray(decision?.actions) ? decision.actions.length : null,
  };
}

// Worker-side perf, plus the two growth signals that a single slow call cannot
// explain on its own: how big the engine's linear memory has become and how
// many cards it has compiled into its registry.
function workerPerf(result) {
  const perf = result && typeof result === "object" ? result.__perf : null;
  if (!perf) return null;
  return {
    method: perf.method ?? null,
    queueWaitMs: perf.queueWaitMs ?? null,
    wasmCallMs: perf.wasmCallMs ?? null,
    totalWorkerMs: perf.totalWorkerMs ?? null,
    estimatedEngineMs: perf.estimatedEngineMs ?? null,
    engineMemoryBytes: perf.engineMemoryBytes ?? null,
    registrySize: perf.registrySize ?? null,
  };
}

export function setJournalPolicy(policy) {
  store.policy = policy === "redacted" ? "redacted" : "full";
}

export function journalPolicy() {
  return store.policy;
}

export function recordWorkerInit(info) {
  store.workerInit = safeClone(info);
}

/**
 * Card definitions outside the baked registry are fetched and registered by the
 * worker rather than by an engine call, so they are invisible to the journal.
 * Recording their routes lets a replay register the same set before it starts.
 */
export function recordCardRoutes(routes) {
  store.cardRoutes = Array.isArray(routes) ? routes.map(String) : null;
}

/**
 * Open an entry for a worker call. Returns null for reads and for calls that
 * arrive after the journal has overflowed, in which case the caller's
 * completion helpers are no-ops.
 */
export function beginJournalEntry(method, args) {
  if (!isMutation(method)) return null;
  if (store.overflowed) return null;
  if (store.entries.length >= MAX_ENTRIES) {
    store.overflowed = true;
    store.overflowAtSeq = store.sequence + 1;
    return null;
  }

  const startedAt = now();
  if (store.startedAtMs == null) {
    store.startedAtMs = startedAt;
    store.startedAtWall = Date.now();
  }

  store.sequence += 1;
  const entry = {
    seq: store.sequence,
    method,
    args: null,
    argsOmitted: false,
    argShape: null,
    atMs: startedAt,
    durationMs: null,
    worker: null,
    after: null,
    error: null,
  };

  const capture = store.policy === "full" && !ARGS_NEVER_CAPTURED.has(method);
  if (capture) {
    const { clone: cloned, bytes } = cloneWithSize(args);
    if (cloned == null || bytes > MAX_ENTRY_ARG_BYTES || store.argBytes + bytes > MAX_TOTAL_ARG_BYTES) {
      store.overflowed = true;
      store.overflowAtSeq = entry.seq;
      entry.argsOmitted = true;
      entry.argShape = argListShape(args);
    } else {
      store.argBytes += bytes;
      entry.args = cloned;
    }
  } else {
    entry.argsOmitted = true;
    entry.argShape = argListShape(args);
  }

  store.entries.push(entry);
  return entry;
}

export function completeJournalEntry(entry, result) {
  if (!entry) return;
  entry.durationMs = now() - entry.atMs;
  entry.worker = workerPerf(result);
  entry.after = stateFingerprint(result);
}

export function failJournalEntry(entry, error) {
  if (!entry) return;
  entry.durationMs = now() - entry.atMs;
  entry.error = String(error?.message || error);
}

export function getJournal() {
  return {
    version: JOURNAL_VERSION,
    policy: store.policy,
    // A journal whose arguments were dropped, or that stopped recording before
    // the session ended, describes a session it cannot rebuild. Say so here
    // rather than letting a harness discover it halfway through a replay.
    replayable: !store.overflowed && store.entries.every((entry) => !entry.argsOmitted),
    overflowed: store.overflowed,
    overflowAtSeq: store.overflowAtSeq,
    startedAtMs: store.startedAtMs,
    startedAtWall: store.startedAtWall,
    entryCount: store.entries.length,
    approxArgBytes: store.argBytes,
    workerInit: store.workerInit,
    cardRoutes: store.cardRoutes,
    entries: store.entries,
  };
}

export function journalSummary() {
  const journal = getJournal();
  return {
    version: journal.version,
    policy: journal.policy,
    replayable: journal.replayable,
    overflowed: journal.overflowed,
    entryCount: journal.entryCount,
    approxArgBytes: journal.approxArgBytes,
  };
}

export function resetJournal() {
  store.entries = [];
  store.sequence = 0;
  store.startedAtMs = null;
  store.startedAtWall = null;
  store.argBytes = 0;
  store.overflowed = false;
  store.overflowAtSeq = null;
  store.workerInit = null;
  store.cardRoutes = null;
}
