import test from "node:test";
import assert from "node:assert/strict";
import {
  beginJournalEntry,
  completeJournalEntry,
  failJournalEntry,
  getJournal,
  journalSummary,
  recordCardRoutes,
  recordWorkerInit,
  resetJournal,
  setJournalPolicy,
} from "../src/lib/engine-journal.js";

function snapshotResult(overrides = {}) {
  return {
    snapshot_id: 7,
    turn_number: 5,
    phase: "beginning phase",
    step: "upkeep step",
    active_player: 0,
    priority_player: 1,
    stack_size: 1,
    battlefield_size: 26,
    exile_size: 9,
    decision: { kind: "priority", player: 1, analysis_complete: false, actions: [{ index: 0 }] },
    __perf: { method: "dispatch", wasmCallMs: 119596.3, totalWorkerMs: 119596.9, engineMemoryBytes: 420 * 1048576, registrySize: 1234 },
    ...overrides,
  };
}

test.beforeEach(() => {
  resetJournal();
  setJournalPolicy("full");
});

test("mutations are journaled in order and reads are skipped", () => {
  assert.equal(beginJournalEntry("uiState", []), null, "a read cannot move the engine");
  assert.equal(beginJournalEntry("snapshot", []), null);
  assert.equal(beginJournalEntry("lastDispatchPerf", []), null);

  const first = beginJournalEntry("startMatch", [{ seed: "abc", decks: [["Island"]] }]);
  completeJournalEntry(first, null);
  const second = beginJournalEntry("dispatch", [{ type: "priority_action", action_index: 0 }]);
  completeJournalEntry(second, snapshotResult());

  const journal = getJournal();
  assert.deepEqual(journal.entries.map((entry) => [entry.seq, entry.method]), [
    [1, "startMatch"],
    [2, "dispatch"],
  ]);
  assert.equal(journal.replayable, true);
});

test("an entry carries the arguments and the state the call produced", () => {
  const entry = beginJournalEntry("dispatch", [{ type: "priority_action", action_index: 0 }]);
  completeJournalEntry(entry, snapshotResult());

  const [recorded] = getJournal().entries;
  assert.deepEqual(recorded.args, [{ type: "priority_action", action_index: 0 }]);
  assert.equal(recorded.argsOmitted, false);
  assert.ok(recorded.durationMs >= 0);
  // Fingerprint, so a replay that lands somewhere else is caught at this entry.
  assert.equal(recorded.after.turn, 5);
  assert.equal(recorded.after.stackSize, 1);
  assert.equal(recorded.after.battlefieldSize, 26);
  assert.equal(recorded.after.decisionKind, "priority");
  assert.equal(recorded.after.analysisComplete, false);
  // Growth signals a single slow call cannot supply on its own.
  assert.equal(recorded.worker.wasmCallMs, 119596.3);
  assert.equal(recorded.worker.engineMemoryBytes, 420 * 1048576);
  assert.equal(recorded.worker.registrySize, 1234);
});

test("arguments mutated after the call are not retroactively rewritten", () => {
  const args = [{ type: "priority_action", action_index: 0 }];
  const entry = beginJournalEntry("dispatch", args);
  args[0].action_index = 99;
  completeJournalEntry(entry, snapshotResult());
  assert.equal(getJournal().entries[0].args[0].action_index, 0);
});

test("a failed call is recorded and excluded from nothing", () => {
  const entry = beginJournalEntry("dispatch", [{ type: "select_targets" }]);
  failJournalEntry(entry, new Error("no pending decision to dispatch"));
  const [recorded] = getJournal().entries;
  assert.equal(recorded.error, "no pending decision to dispatch");
  assert.equal(recorded.after, null);
});

test("a redacted journal withholds arguments and refuses to claim it is replayable", () => {
  setJournalPolicy("redacted");
  const entry = beginJournalEntry("startMatch", [{ seed: "abc", decks: [["Island", "Mountain"]] }]);
  completeJournalEntry(entry, null);

  const journal = getJournal();
  const [recorded] = journal.entries;
  assert.equal(recorded.args, null);
  assert.equal(recorded.argsOmitted, true);
  // The shape survives so a reader can still tell an empty deck from a full one.
  assert.equal(recorded.argShape[0].seed, "string(3)");
  assert.equal(journal.replayable, false, "a peer match cannot be replayed from method names");
});

test("peer transport arguments are never captured even under the full policy", () => {
  const entry = beginJournalEntry("importSyncCheckpoint", [{ blob: "other players' hidden state" }]);
  completeJournalEntry(entry, null);
  const [recorded] = getJournal().entries;
  assert.equal(recorded.args, null);
  assert.equal(recorded.argsOmitted, true);
});

test("overflow truncates the tail and leaves the prefix intact", () => {
  const good = beginJournalEntry("dispatch", [{ type: "priority_action", action_index: 0 }]);
  completeJournalEntry(good, snapshotResult());

  // One argument past the per-entry cap trips overflow.
  const huge = beginJournalEntry("addCardToZone", [0, "x".repeat(5 * 1024 * 1024), "battlefield"]);
  completeJournalEntry(huge, null);

  assert.equal(beginJournalEntry("dispatch", [{ type: "priority_action" }]), null,
    "recording stops rather than punching a hole in the middle of the journal");

  const journal = getJournal();
  assert.equal(journal.overflowed, true);
  assert.equal(journal.overflowAtSeq, 2);
  assert.equal(journal.replayable, false);
  assert.deepEqual(journal.entries[0].args, [{ type: "priority_action", action_index: 0 }],
    "everything before the overflow still replays");
});

test("the journal reports the preamble a replay needs before it can start", () => {
  recordWorkerInit({ assetBaseUrl: "https://example.test/" });
  recordCardRoutes(["agatha-s-soul-cauldron", "badgermole-cub"]);
  const journal = getJournal();
  assert.equal(journal.workerInit.assetBaseUrl, "https://example.test/");
  assert.deepEqual(journal.cardRoutes, ["agatha-s-soul-cauldron", "badgermole-cub"]);
});

test("the summary matches the journal and reset clears everything", () => {
  const entry = beginJournalEntry("dispatch", [{ type: "priority_action" }]);
  completeJournalEntry(entry, snapshotResult());
  recordCardRoutes(["island"]);

  const summary = journalSummary();
  assert.equal(summary.entryCount, 1);
  assert.equal(summary.replayable, true);
  assert.ok(summary.approxArgBytes > 0);
  assert.equal(summary.entries, undefined, "the summary must stay cheap enough to render every second");

  resetJournal();
  const cleared = getJournal();
  assert.equal(cleared.entryCount, 0);
  assert.equal(cleared.cardRoutes, null);
  assert.equal(cleared.approxArgBytes, 0);
});
