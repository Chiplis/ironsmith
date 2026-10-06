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
  const invalid = [14, 16, 17, 18, null, "19"].map(version => transcript(version));
  invalid.push({}, { match: {} }, transcript(19, 18), transcript(18, 19), transcript(19, null));
  for (const candidate of invalid) {
    for (const entry of entries) {
      const h = replayGame();
      await assert.rejects(entry({ game: h.game, transcript: candidate, cryptoImpl: webcrypto }), /requires audit protocol 19/);
      assert.deepEqual(h.calls, []);
    }
    let callbacks = 0;
    await assert.rejects(verifyLiveAuditTranscript(candidate, webcrypto, {
      requireEngineReplay: false, replayTranscript: async () => { callbacks++; },
    }), /requires audit protocol 19/);
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
  candidate.match.protocolVersion = 18;
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /requires audit protocol 19/);
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

test("current replay requires v3 checkpoint exports before start and per-action mutation", async () => {
  for (const version of [undefined, 2, "3"]) {
    for (const entry of [startAuditTranscriptReplayWithGame, replayAuditTranscriptWithGame,
      verifyEndOfMatchDisclosuresWithGame]) {
      const h = replayGame();
      h.setCheckpoint({ version });
      await assert.rejects(entry({ game: h.game, transcript: transcript() }), /checkpoint version 3/);
      assert.deepEqual(h.calls, ["export"]);
    }
  }
  const h = replayGame();
  await startAuditTranscriptReplayWithGame({ game: h.game, transcript: transcript(), cryptoImpl: webcrypto });
  h.calls.length = 0;
  h.setCheckpoint({ version: 2 });
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /checkpoint version 3/);
  assert.deepEqual(h.calls, ["export"]);
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
  assert.deepEqual(await h.game.exportPublicAuditCheckpoint(), { version: 3, live: true });
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
