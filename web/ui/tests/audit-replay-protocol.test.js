import test from "node:test";
import assert from "node:assert/strict";
import { createHash, webcrypto } from "node:crypto";
import {
  applyAuditReplayActionWithGame,
  replayAuditTranscriptWithGame,
  startAuditTranscriptReplayWithGame,
  verifyEndOfMatchDisclosuresWithGame,
} from "../src/lib/audit-replay.js";
import {
  CURRENT_AUDIT_PROTOCOL_VERSION,
  CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION,
  publicCheckpointHash,
  verifyLiveAuditTranscript,
} from "../src/lib/multiplayer-audit.js";

function transcript(protocolVersion = CURRENT_AUDIT_PROTOCOL_VERSION, matchVersion = protocolVersion) {
  return { protocolVersion, match: { protocolVersion: matchVersion, players: [] }, actions: [] };
}

function replayGame() {
  const calls = [];
  let checkpoint = { version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION, live: true };
  let saved;
  const game = {
    exportPublicAuditCheckpoint: async () => { calls.push("export"); return checkpoint; },
    getHiddenCardState: async () => { calls.push("hidden"); return { objects: [] }; },
    createRuntimeSavepoint: async () => { calls.push("save"); saved = checkpoint; return 1; },
    restoreRuntimeSavepoint: async () => { calls.push("restore"); checkpoint = saved; },
    startMatch: async () => { calls.push("start"); checkpoint = { version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION }; },
    uiState: async () => { calls.push("ui"); return {}; },
    previewCryptoRequirements: async () => { calls.push("preview"); return []; },
    dispatch: async () => { calls.push("dispatch"); },
  };
  return { game, calls, setCheckpoint: value => { checkpoint = value; } };
}

const action = { command: { type: "priority_action", action_ref: { kind: "pass_priority" } } };

test("all engine replay entry points reject old, absent and mismatched protocol before reading engine state", async () => {
  const entries = [replayAuditTranscriptWithGame, startAuditTranscriptReplayWithGame,
    verifyEndOfMatchDisclosuresWithGame];
  const invalid = [14, 16, 17, 18, 19, 20, 21, CURRENT_AUDIT_PROTOCOL_VERSION + 1, null, String(CURRENT_AUDIT_PROTOCOL_VERSION)].map(version => transcript(version));
  invalid.push({}, { match: {} }, transcript(CURRENT_AUDIT_PROTOCOL_VERSION, 20), transcript(20, CURRENT_AUDIT_PROTOCOL_VERSION), transcript(CURRENT_AUDIT_PROTOCOL_VERSION, 21), transcript(21, CURRENT_AUDIT_PROTOCOL_VERSION), transcript(CURRENT_AUDIT_PROTOCOL_VERSION, null));
  for (const candidate of invalid) {
    for (const entry of entries) {
      const h = replayGame();
      await assert.rejects(entry({ game: h.game, transcript: candidate, cryptoImpl: webcrypto }), new RegExp(`requires audit protocol ${CURRENT_AUDIT_PROTOCOL_VERSION}`));
      assert.deepEqual(h.calls, []);
    }
    let callbacks = 0;
    await assert.rejects(verifyLiveAuditTranscript(candidate, webcrypto, {
      requireEngineReplay: false, replayTranscript: async () => { callbacks++; },
    }), new RegExp(`requires audit protocol ${CURRENT_AUDIT_PROTOCOL_VERSION}`));
    assert.equal(callbacks, 0);
  }
});

test("actions require successful initialization and recheck the session protocol before engine work", async () => {
  const h = replayGame();
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /successfully initialized/);
  assert.deepEqual(h.calls, []);
  const candidate = transcript();
  await startAuditTranscriptReplayWithGame({ game: h.game, transcript: candidate, cryptoImpl: webcrypto });
  h.calls.length = 0;
  candidate.match.protocolVersion = 21;
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), new RegExp(`requires audit protocol ${CURRENT_AUDIT_PROTOCOL_VERSION}`));
  assert.deepEqual(h.calls, []);
});

test("a failed initial checkpoint comparison cannot authorize a later action", async () => {
  const h = replayGame();
  await assert.rejects(startAuditTranscriptReplayWithGame({ game: h.game,
    transcript: { ...transcript(), initialPublicCheckpointHash: "wrong" }, cryptoImpl: webcrypto }), /initial public checkpoint/);
  h.calls.length = 0;
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /successfully initialized/);
  assert.deepEqual(h.calls, []);
});

test("current replay requires v5 checkpoint exports before start and per-action mutation", async () => {
  for (const version of [undefined, 2, 3, 4, "5"]) {
    for (const entry of [startAuditTranscriptReplayWithGame, replayAuditTranscriptWithGame,
      verifyEndOfMatchDisclosuresWithGame]) {
      const h = replayGame();
      h.setCheckpoint({ version });
      await assert.rejects(entry({ game: h.game, transcript: transcript() }), /checkpoint version 5/);
      assert.deepEqual(h.calls, ["export"]);
    }
  }
  const h = replayGame();
  await startAuditTranscriptReplayWithGame({ game: h.game, transcript: transcript(), cryptoImpl: webcrypto });
  h.calls.length = 0;
  h.setCheckpoint({ version: 4 });
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /checkpoint version 5/);
  assert.deepEqual(h.calls, ["export"]);
});

test("current signed replay refuses a historical final digest before invoking its replay callback", async () => {
  let callbacks = 0;
  await assert.rejects(verifyLiveAuditTranscript({ ...transcript(), finalPublicCheckpoint: { version: 4 } }, webcrypto, {
    requireEngineReplay: false, replayTranscript: async () => { callbacks++; },
  }), /checkpoint version 5/);
  assert.equal(callbacks, 0);
});

test("a failed action cannot keep authorizing dispatches in a partially changed replay", async () => {
  const h = replayGame();
  await startAuditTranscriptReplayWithGame({ game: h.game, transcript: transcript(), cryptoImpl: webcrypto });
  h.game.dispatch = async () => { throw new Error("action failed"); };
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /action failed/);
  h.calls.length = 0;
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /successfully initialized/);
  assert.deepEqual(h.calls, []);
});

test("full replay restores a previously successful current session after an initial hash failure", async () => {
  const h = replayGame();
  await startAuditTranscriptReplayWithGame({ game: h.game, transcript: transcript(), cryptoImpl: webcrypto });
  await assert.rejects(replayAuditTranscriptWithGame({ game: h.game,
    transcript: { ...transcript(), initialPublicCheckpointHash: "wrong" }, cryptoImpl: webcrypto }), /initial public checkpoint/);
  h.calls.length = 0;
  await applyAuditReplayActionWithGame({ game: h.game, action, cryptoImpl: webcrypto });
  assert.ok(h.calls.includes("dispatch"));
});

test("a failed runtime restoration cannot revive a previous replay session", async () => {
  const h = replayGame();
  await startAuditTranscriptReplayWithGame({ game: h.game, transcript: transcript(), cryptoImpl: webcrypto });
  h.game.restoreRuntimeSavepoint = async () => { throw new Error("restore failed"); };
  await assert.rejects(replayAuditTranscriptWithGame({ game: h.game, transcript: transcript(), cryptoImpl: webcrypto }), /restore failed/);
  h.calls.length = 0;
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /successfully initialized/);
  assert.deepEqual(h.calls, []);
});

test("current replay restores the caller and revokes its temporary action session", async () => {
  const h = replayGame();
  const report = await replayAuditTranscriptWithGame({ game: h.game, transcript: transcript(), cryptoImpl: webcrypto });
  assert.equal(report.verified, true);
  assert.ok(h.calls.includes("restore"));
  assert.deepEqual(await h.game.exportPublicAuditCheckpoint(), { version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION, live: true });
  h.calls.length = 0;
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /successfully initialized/);
  assert.deepEqual(h.calls, []);
});

test("historical checkpoint hashing preserves the v2 payload without injecting cloak defaults", async () => {
  const old = { version: 2, players: [], stack: [], objects: [{ id: 7, stableId: 7, manifested: true }] };
  const oldCanonical = '{"checkpoint":{"objects":[{"id":7,"manifested":true,"stableId":7}],"players":[],"stack":[],"version":2},"domain":"ironsmith-public-audit-checkpoint-v1"}';
  const expected = createHash("sha256").update(oldCanonical).digest("hex");
  assert.equal(await publicCheckpointHash(old, webcrypto), expected);
  assert.equal(Object.hasOwn(old.objects[0], "cloaked"), false);
  assert.equal(old.version, 2);
  const manifested = { ...old, version: 3, objects: [{ ...old.objects[0], cloaked: false }] };
  const cloaked = { ...manifested, objects: [{ ...manifested.objects[0], manifested: false, cloaked: true }] };
  assert.notEqual(await publicCheckpointHash(manifested, webcrypto), await publicCheckpointHash(cloaked, webcrypto));
});

test("historical v3 hashing preserves original bytes without injecting numeric proof", async () => {
  const old = { version: 3, players: [], stack: [], objects: [
    { id: 7, stableId: 7, manifested: false, cloaked: false },
  ] };
  const before = structuredClone(old);
  const oldCanonical = '{"checkpoint":{"objects":[{"cloaked":false,"id":7,"manifested":false,"stableId":7}],"players":[],"stack":[],"version":3},"domain":"ironsmith-public-audit-checkpoint-v1"}';
  assert.equal(await publicCheckpointHash(old, webcrypto), createHash("sha256").update(oldCanonical).digest("hex"));
  assert.deepEqual(old, before);
  assert.equal(Object.hasOwn(old.objects[0], "numericChoices"), false);
  const definition = Array(32).fill(7);
  const current = { ...old, version: 4, objects: [{ ...old.objects[0], numericChoices: {
    records: [], bindings: [{ slot: 0, definition, pair: 0, group: null }],
  } }] };
  const chosenZero = structuredClone(current);
  chosenZero.objects[0].numericChoices.records.push({ group: 0, definition, pair: 0, number: 0 });
  chosenZero.objects[0].numericChoices.bindings[0].group = 0;
  assert.notEqual(await publicCheckpointHash(current, webcrypto), await publicCheckpointHash(chosenZero, webcrypto));
  const chosenLarge = structuredClone(chosenZero);
  chosenLarge.objects[0].numericChoices.records[0].number = 4294967295;
  assert.notEqual(await publicCheckpointHash(chosenZero, webcrypto), await publicCheckpointHash(chosenLarge, webcrypto));
});


test("historical v4 hashing preserves nested claim commitments without prepared snapshot defaults", async () => {
  const historicalContext = '{"effectOutcomes":[[1,{"status":"Success","value":{"Count":0},"execution_facts":[]}]]}';
  const ledgerDigest = createHash("sha256").update(historicalContext).digest("hex");
  const checkpoint = { version: 4, hiddenClaimLedgerDigest: ledgerDigest,
    players: [], stack: [], objects: [] };
  const before = structuredClone(checkpoint);
  const canonical = '{"checkpoint":{"hiddenClaimLedgerDigest":"' + ledgerDigest
    + '","objects":[],"players":[],"stack":[],"version":4},"domain":"ironsmith-public-audit-checkpoint-v1"}';
  assert.equal(await publicCheckpointHash(checkpoint, webcrypto),
    createHash("sha256").update(canonical).digest("hex"));
  assert.deepEqual(checkpoint, before);
  assert.equal(historicalContext.includes("stack_kind"), false);
  assert.notEqual(await publicCheckpointHash(checkpoint, webcrypto),
    await publicCheckpointHash({ ...checkpoint, version: 5 }, webcrypto));
  const empty = { version: 4, players: [], stack: [], objects: [] };
  assert.notEqual(await publicCheckpointHash(empty, webcrypto),
    await publicCheckpointHash({ ...empty, hiddenClaimLedgerDigest: ledgerDigest }, webcrypto));
  assert.equal(Object.hasOwn(empty, "hiddenClaimLedgerDigest"), false);
});
