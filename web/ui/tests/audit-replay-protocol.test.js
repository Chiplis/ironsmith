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

// Source-authored, UNRUN. An independent canonical string keeps digest10
// evidence independent of current Rust model defaults and release labels.
test("historical digest10 retains its canonical bytes across digest11 admission", async () => {
  const checkpoint = { version: 10, players: [], objects: [], stack: [],
    lastAttackDeclarationStepPlayers: [],
    grandMelee: { markers: [{ combat: { lastAttackDeclarationStepPlayers: null } }] } };
  const before = structuredClone(checkpoint);
  const canonical = '{"checkpoint":{"grandMelee":{"markers":[{"combat":{"lastAttackDeclarationStepPlayers":null}}]},"lastAttackDeclarationStepPlayers":[],"objects":[],"players":[],"stack":[],"version":10},"domain":"ironsmith-public-audit-checkpoint-v1"}';
  const expected = createHash("sha256").update(canonical).digest("hex");
  assert.equal(await publicCheckpointHash(checkpoint, webcrypto), expected);
  assert.deepEqual(checkpoint, before);
  assert.notEqual(await publicCheckpointHash({ ...checkpoint, version: 11 }, webcrypto), expected);
});

test("all engine replay entry points reject old, absent and mismatched protocol before reading engine state", async () => {
  const entries = [replayAuditTranscriptWithGame, startAuditTranscriptReplayWithGame,
    verifyEndOfMatchDisclosuresWithGame];
  const invalid = [14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, CURRENT_AUDIT_PROTOCOL_VERSION + 1, null, String(CURRENT_AUDIT_PROTOCOL_VERSION)].map(version => transcript(version));
  for (const version of [26, 27, 28, 29, 30, undefined, null, String(CURRENT_AUDIT_PROTOCOL_VERSION)]) {
    invalid.push({ protocolVersion: version, match: { protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION } });
    invalid.push({ protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION, match: { protocolVersion: version } });
  }
  invalid.push({}, { match: {} }, transcript(CURRENT_AUDIT_PROTOCOL_VERSION, 20), transcript(20, CURRENT_AUDIT_PROTOCOL_VERSION), transcript(CURRENT_AUDIT_PROTOCOL_VERSION, 21), transcript(21, CURRENT_AUDIT_PROTOCOL_VERSION), transcript(CURRENT_AUDIT_PROTOCOL_VERSION, 22), transcript(22, CURRENT_AUDIT_PROTOCOL_VERSION), transcript(CURRENT_AUDIT_PROTOCOL_VERSION, 23), transcript(23, CURRENT_AUDIT_PROTOCOL_VERSION), transcript(CURRENT_AUDIT_PROTOCOL_VERSION, 24), transcript(24, CURRENT_AUDIT_PROTOCOL_VERSION), transcript(CURRENT_AUDIT_PROTOCOL_VERSION, 25), transcript(25, CURRENT_AUDIT_PROTOCOL_VERSION), transcript(CURRENT_AUDIT_PROTOCOL_VERSION, null));
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
  for (const version of [25, 26, 27, 28, 29, 30, undefined, null, String(CURRENT_AUDIT_PROTOCOL_VERSION)]) {
    for (const owner of ["transcript", "match"]) {
      const candidate = transcript();
      await startAuditTranscriptReplayWithGame({ game: h.game, transcript: candidate, cryptoImpl: webcrypto });
      h.calls.length = 0;
      (owner === "match" ? candidate.match : candidate).protocolVersion = version;
      await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), new RegExp(`requires audit protocol ${CURRENT_AUDIT_PROTOCOL_VERSION}`));
      assert.deepEqual(h.calls, []);
    }
  }
});

test("a failed initial checkpoint comparison cannot authorize a later action", async () => {
  const h = replayGame();
  await assert.rejects(startAuditTranscriptReplayWithGame({ game: h.game,
    transcript: { ...transcript(), initialPublicCheckpointHash: "wrong" }, cryptoImpl: webcrypto }), /initial public checkpoint/);
  h.calls.length = 0;
  await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /successfully initialized/);
  assert.deepEqual(h.calls, []);
});

test("current replay requires v11 checkpoint exports before start and per-action mutation", async () => {
  for (const version of [undefined, null, 2, 3, 4, 5, 6, 7, 8, 9, 10, "9", "10", "11", 12]) {
    for (const entry of [startAuditTranscriptReplayWithGame, replayAuditTranscriptWithGame,
      verifyEndOfMatchDisclosuresWithGame]) {
      const h = replayGame();
      h.setCheckpoint({ version });
      await assert.rejects(entry({ game: h.game, transcript: transcript() }), /checkpoint version 11/);
      assert.deepEqual(h.calls, ["export"]);
    }
  }
  for (const version of [undefined, null, 7, 8, 9, 10, "9", "10", "11", 12]) {
    const h = replayGame();
    await startAuditTranscriptReplayWithGame({ game: h.game, transcript: transcript(), cryptoImpl: webcrypto });
    h.calls.length = 0;
    h.setCheckpoint({ version });
    await assert.rejects(applyAuditReplayActionWithGame({ game: h.game, action }), /checkpoint version 11/);
    assert.deepEqual(h.calls, ["export"]);
  }
});

test("current signed replay refuses a historical final digest before invoking its replay callback", async () => {
  let callbacks = 0;
  for (const version of [undefined, null, 7, 8, 9, 10, "9", "10", "11", 12]) {
    await assert.rejects(verifyLiveAuditTranscript({ ...transcript(), finalPublicCheckpoint: { version } }, webcrypto, {
      requireEngineReplay: false, replayTranscript: async () => { callbacks++; },
    }), /checkpoint version 11/);
  }
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


test("synthetic historical v5 typed mana-program hashing preserves canonical bytes", async () => {
  // Authored from the v5 model shape, not a captured historical artifact.
  const gain = { kind: "GainLifeEffect", payload: { amount: { Fixed: 2 }, player: { Player: "You" } } };
  const checkpoint = { version: 5, players: [{ id: 0, restrictedMana: [{
    symbol: "Green", source: 17, source_controller: 0, source_chosen_creature_type: null,
    restrictions: [{ PaymentTransaction: { restriction: "Any", on_spend: [{
      predicate: "Any", effects: {
        segments: [{ default_effects: [gain], self_replacements: [], starts_new_source_line: false }],
        flattened_default_effects: [gain],
      }, choices: [],
    }] } }],
  }] }], stack: [], objects: [] };
  const canonical = '{"checkpoint":{"objects":[],"players":[{"id":0,"restrictedMana":[{"restrictions":[{"PaymentTransaction":{"on_spend":[{"choices":[],"effects":{"flattened_default_effects":[{"kind":"GainLifeEffect","payload":{"amount":{"Fixed":2},"player":{"Player":"You"}}}],"segments":[{"default_effects":[{"kind":"GainLifeEffect","payload":{"amount":{"Fixed":2},"player":{"Player":"You"}}}],"self_replacements":[],"starts_new_source_line":false}]},"predicate":"Any"}],"restriction":"Any"}}],"source":17,"source_chosen_creature_type":null,"source_controller":0,"symbol":"Green"}]}],"stack":[],"version":5},"domain":"ironsmith-public-audit-checkpoint-v1"}';
  const expected = createHash("sha256").update(canonical).digest("hex");
  const before = structuredClone(checkpoint);
  assert.equal(await publicCheckpointHash(checkpoint, webcrypto), expected);
  assert.deepEqual(checkpoint, before);
  assert.notEqual(await publicCheckpointHash({ ...checkpoint, version: 6 }, webcrypto), expected);
  const changed = structuredClone(checkpoint);
  const program = changed.players[0].restrictedMana[0].restrictions[0].PaymentTransaction.on_spend[0].effects;
  program.segments[0].default_effects[0].payload.amount.Fixed = 3;
  program.flattened_default_effects[0].payload.amount.Fixed = 3;
  assert.notEqual(await publicCheckpointHash(changed, webcrypto), expected);
  assert.deepEqual(checkpoint, before, "hashing cannot migrate historical programs");
});

// Source-authored, UNRUN. Synthetic v7 model bytes, not a recovered historical
// golden. Historical verification must not run the new Rust model defaults.
test("historical v7 typed prevention programs retain absent fields and the original hash domain", async () => {
  const finite = { kind: "PreventDamageEffect", payload: {
    amount: { Fixed: 4 }, target: "Source", until: "EndOfTurn", follow_up_effects: [],
    source_of_your_choice: false, protect_you_and_permanents_you_control: false,
  } };
  const unlimited = { kind: "PreventAllDamageToTargetEffect", payload: {
    target: "Source", until: "EndOfTurn", follow_up_effects: [], combat_only: true,
  } };
  const checkpoint = { version: 7, players: [{ id: 0, restrictedMana: [{
    symbol: "Green", source: 17, source_controller: 0, source_chosen_creature_type: null,
    restrictions: [{ PaymentTransaction: { restriction: "Any", on_spend: [{
      predicate: "Any", effects: {
        segments: [{ default_effects: [finite, unlimited], self_replacements: [], starts_new_source_line: false }],
        flattened_default_effects: [finite, unlimited],
      }, choices: [],
    }] } }],
  }] }], stack: [], objects: [], hiddenClaimLedgerDigest: "7".repeat(64) };
  // Independently canonicalize these already public synthetic rows. This does
  // not call the production checkpoint normalizer or decode a Rust model.
  const canonical = value => Array.isArray(value) ? value.map(canonical)
    : value && typeof value === "object"
      ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])])) : value;
  const expected = createHash("sha256").update(JSON.stringify(canonical({
    checkpoint, domain: "ironsmith-public-audit-checkpoint-v1",
  }))).digest("hex");
  const before = structuredClone(checkpoint);
  assert.equal(await publicCheckpointHash(checkpoint, webcrypto), expected);
  assert.deepEqual(checkpoint, before);
  assert.equal(Object.hasOwn(finite.payload, "damage_filter"), false);
  assert.equal(Object.hasOwn(unlimited.payload, "damage_filter"), false);
  assert.equal(Object.hasOwn(unlimited.payload, "source_color_of_your_choice"), false);
  assert.notEqual(await publicCheckpointHash({ ...checkpoint, version: 8 }, webcrypto), expected);
  for (const [index, key, value] of [
    [0, "damage_filter", { combat_only: false, noncombat_only: false }],
    [1, "damage_filter", { combat_only: false, noncombat_only: false }],
    [1, "source_color_of_your_choice", false],
    [1, "source_color_of_your_choice", true],
  ]) {
    const changed = structuredClone(checkpoint);
    const program = changed.players[0].restrictedMana[0].restrictions[0].PaymentTransaction.on_spend[0].effects;
    program.segments[0].default_effects[index].payload[key] = value;
    program.flattened_default_effects[index].payload[key] = value;
    assert.notEqual(await publicCheckpointHash(changed, webcrypto), expected);
  }
  assert.deepEqual(checkpoint, before);
});
