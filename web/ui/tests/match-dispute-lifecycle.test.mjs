import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { assertMatchNotDisputed, compactMatchDisputeEvidence, isMatchDisputed } from "../src/hooks/peer-lobby/match-lifecycle.js";

// Execute actual hook callbacks, injecting only the engine/React/transport
// boundaries. MATCH_DISPUTE_BASELINE_REF=74993e4af selects the pre-fix source.
const source = (file) => process.env.MATCH_DISPUTE_BASELINE_REF
  ? execFileSync("git", ["show", `${process.env.MATCH_DISPUTE_BASELINE_REF}:web/ui/src/hooks/${file}`], { encoding: "utf8" })
  : readFileSync(new URL(`../src/hooks/${file}`, import.meta.url), "utf8");
const lobby = source("usePeerLobby.js");
const crypto = source("peer-lobby/crypto-resync.js");
const validation = source("peer-lobby/validation.js");
const between = (text, begin, end) => {
  const start = text.indexOf(begin);
  const finish = text.indexOf(end, start + begin.length);
  assert.ok(start >= 0 && finish > start, `Missing source boundary ${begin}`);
  return text.slice(start, finish);
};
const compile = (body, context) => new Function(...Object.keys(context), body)(...Object.values(context));
const acceptedClock = () => ({ policy: { initialMs: 1800000 }, playerCount: 2,
  baseRemainingMsByPlayer: [900000, 910000], activePlayerIndex: 0,
  epochStartedAtMs: 100, clockHash: "verified-clock-516", lastSequence: 516 });
const disputed = () => ({ mode: "disputed", matchStarted: false, lastAppliedSequence: 516,
  matchDisputed: { reason: "Protocol response timeout from Alice; two-player matches require external arbitration." } });
// shared.js resolves Vite aliases, so take the real helper from its source.
const protocolResponseTimeoutClaimFromError = compile(between(source("peer-lobby/shared.js"),
  "export function protocolResponseTimeoutClaimFromError(", "\nexport ").replace(/^export /, "")
  + "\nreturn protocolResponseTimeoutClaimFromError;", {});
// Host-only verified-resync checkpoint capture (crypto-resync.js) has no
// bearing on dispute handling, so the harness makes it a no-op.
const helpers = { assertMatchNotDisputed, compactMatchDisputeEvidence, isMatchDisputed,
  // Optional protocol-order/optimistic-state services live outside these
  // extracted lifecycle functions; individual scenarios can override them.
  servicesRef: { current: {} },
  protocolResponseTimeoutClaimFromError, captureResyncReplayCheckpointIfDue() {} };

test("dispute stops the clock effect without erasing locally accepted hash, balance, or sequence; idle still resets", () => {
  const effect = process.env.MATCH_DISPUTE_BASELINE_REF ? "" : between(lobby, "  useEffect(() => {\n    // A dispute", "    let disposed = false;");
  // The old effect starts at its condition; retain a reproducible baseline.
  const baselineEffect = process.env.MATCH_DISPUTE_BASELINE_REF
    ? between(lobby, "  useEffect(() => {\n    if (!multiplayer.matchStarted ||", "    let disposed = false;") : effect;
  const run = (session) => {
    const clock = acceptedClock();
    const ref = { current: structuredClone(clock) };
    compile(baselineEffect + "  });", { ...helpers,
      useEffect: (fn) => fn(), multiplayer: session, multiplayerRef: { current: session },
      matchClockConfigRef: { current: clock.policy }, matchClockRef: ref,
      createMatchClockSnapshot: () => ({ enabled: true, initialMs: 1800000 }),
      INITIAL_MATCH_CLOCK_HASH: "zero", timeoutClaimInFlightRef: { current: "" },
      actionTimerSnapshotFromMatchClock: (value) => value,
      updateMultiplayer: (fn) => fn(session),
    });
    return ref.current;
  };
  assert.deepEqual(run(disputed()), acceptedClock());
  assert.equal(run({ mode: "lobby", matchStarted: false }).clockHash, "zero");
});

test("original dispute reason/evidence survives later failures and diagnostics omit private request payloads", () => {
  const session = { current: { matchStarted: true, mode: "in_match", lastAppliedSequence: 516 } };
  const transcript = { current: { actions: [], disputes: [] } };
  const pending = { current: new Map([[517, { secret: "not-for-export" }]]) };
  const events = [];
  const mark = compile(between(crypto, "  function markMatchDisputed(", "  async function handleHistoricalSequencedAction") + "return markMatchDisputed;", {
    ...helpers, multiplayerRef: session, matchClockRef: { current: acceptedClock() },
    frozenAcceptedMatchClockRuntime: () => ({ ...acceptedClock(), activePlayerIndex: null, epochStartedAtMs: null }),
    matchClockConfigRef: { current: {} }, runtimeMatchClockSnapshot: () => acceptedClock(),
    actionTimerSnapshotFromMatchClock: (value) => value,
    INITIAL_MATCH_CLOCK_HASH: "zero", liveAuditTranscriptRef: transcript,
    pendingSequencedActionsRef: pending, timeoutClaimInFlightRef: { current: "" },
    cloneMultiplayerPayload: structuredClone, ignoreAndClearAllPendingActionIntents() {},
    emitSyncFailureNotice() {}, setStatus() {}, recordDiagnosticEvent: (...event) => events.push(event),
    updateMultiplayer: (fn) => { session.current = fn(session.current); },
  });
  mark(disputed().matchDisputed.reason, { type: "protocol_response_timeout", accusedPlayers: [0],
    claim: { basisSequence: 516, requestType: "action_intent", responseTimeoutMs: 120000,
      requestPayload: { card: "PRIVATE HAND CARD", nonce: "SECRET NONCE" } } });
  mark("Match clock hash chain does not match local transcript", {});
  assert.equal(session.current.matchDisputed.reason, disputed().matchDisputed.reason);
  assert.deepEqual(session.current.matchDisputed.accusedPlayers, [0]);
  assert.equal(events.length, 1);
  assert.equal(events[0][0], "match_disputed");
  assert.equal(events[0][1].clockHash, "verified-clock-516");
  assert.equal(events[0][1].evidence.responseTimeoutMs, 120000);
  assert.doesNotMatch(JSON.stringify(events), /PRIVATE HAND CARD|SECRET NONCE|requestPayload/);
  assert.equal(pending.current.size, 0);
});

test("crypto request distinguishes a disputed session from a match that never started", async () => {
  const authorization = between(crypto, "  const authorizedCryptoMaterialRequirementsForRequest", '    if (String(message?.matchId');
  const run = (session) => compile(authorization + "return [];\n  }); return authorizedCryptoMaterialRequirementsForRequest;", {
    ...helpers, useCallback: (fn) => fn, multiplayerRef: { current: session },
  })({}, {});
  await assert.rejects(run(disputed()), /match disputed.*Protocol response timeout from Alice/);
  await assert.rejects(run({ matchStarted: false }), /before match start/);
  assert.deepEqual(await run({ matchStarted: true, mode: "in_match" }), []);
});

function actionHarness({ pauseAt = null, verified = false } = {}) {
  let engine = 516, applied = 0, commits = 0, releases = 0, restores = 0;
  let resume, reached;
  const paused = new Promise((resolve) => { reached = resolve; });
  const pause = async (phase) => {
    if (pauseAt !== phase) return;
    reached();
    await new Promise((resolve) => { resume = resolve; });
  };
  const multiplayerRef = { current: { mode: "in_match", matchStarted: true, lastAppliedSequence: 516 } };
  const matchClockRef = { current: acceptedClock() };
  const events = [];
  const context = { ...helpers, multiplayerRef,
    actionHistoryEntryForSequence: () => null, awaitingStateResyncRef: { current: false },
    createSequencedActionValidationSnapshot: async () => ({ lastAppliedSequence: 516,
      release: async () => { releases += 1; } }),
    restoreSequencedActionValidationSnapshotIfCurrent: async () => {
      restores += 1; engine = 516; matchClockRef.current = acceptedClock(); return true;
    },
    updateMultiplayer: (fn) => { multiplayerRef.current = fn(multiplayerRef.current); },
    sessionSecurityMode: () => verified ? "verified" : "trusted", matchPayloadSecurityMode: () => verified ? "verified" : "trusted",
    matchStartPayloadRef: { current: {} }, MULTIPLAYER_SECURITY_VERIFIED: "verified",
    MULTIPLAYER_SECURITY_TRUSTED: "trusted", sequencedActionSecurityMode: () => verified ? "verified" : "trusted",
    isTrustedMultiplayerSecurityMode: () => !verified,
    servicesRef: { current: { assertAcceptedActionExtendsTranscript() {} } },
    gameRef: { current: { uiState: async () => ({ sequence: engine }) } },
    validateTrustedSequencedAction: async () => pause("validate"),
    applySyncedCommand: async () => { applied += 1; engine = 517; await pause("engine"); return { sequence: engine }; },
    commitMatchClockAudit: () => { commits += 1; matchClockRef.current.lastSequence = 517; },
    appendAppliedSequencedAction: async () => { multiplayerRef.current.lastAppliedSequence = 517; },
    recordDiagnosticEvent: (...event) => events.push(event),
    matchClockObservationExemptSequenceRef: { current: 0 },
    publishCurrentRuntimeState: async () => {}, relaySequencedAction() {}, drainPendingSequencedActions: async () => {},
    isRejectedActionCheatReason: () => false, isUnauthorizedAddCardCommand: () => false,
    fairRandomRevealLockConflict: () => false, // no random reveal is locked in these scenarios
    summarizePeerCommand: (value) => value, setStatus() {}, console: { error() {} },
    verifySequencedActionAudit: async () => pause("validate"), verifyActionMatchesPendingIntent: async () => ({}),
    verifyActionQuorumForMessage: async () => {}, isActionTimeoutForfeitCommand: () => false,
    isDisconnectTimeoutForfeitCommand: () => false, isProtocolResponseTimeoutForfeitCommand: () => false,
    isWitnessForfeitCommand: () => false, isSelfForfeitCommand: () => false, isForfeitCommand: () => false,
    MATCH_CLOCK_CLAIM_SKEW_MS: 1, MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS: 1,
    verifyMatchClockAuditForAction: async () => {}, isPrivateZiffleEpoch: () => false,
    revealAuditOpenings: async () => null, revealPrivateAuditProofsForLocalViewer: async () => {},
    remapCommandForLocalHiddenOpening: async (command) => command, assertPublicSelectionsOpened: async () => {},
    filterCryptoRequirementsForCommand: (_command, _state, requirements) => requirements,
    freshCryptoRequirementsForSequence: (_seq, requirements) => requirements,
    previewRequirementsForCommand: async () => [], rememberActionCryptoRequirements() {},
    verifyShuffleProofsForRequirements: async () => {}, verifyAuditSatisfiesCryptoRequirements: async () => {},
    injectCryptoMaterialForRequirements: async () => {}, cryptoRequirementsFromState: () => [],
    alignShuffleProofsWithRequirements: () => [], applyVerifiedShuffleProofs: async () => {},
    revealLocalZiffleHand: async () => {}, viewedCardsStateHint: (_remote, appliedState) => appliedState,
    verifyCurrentPublicCheckpointHash: async () => {},
    // The received actions ship no openings, so nothing needs allowing.
    commandObjectIdsForOpeningAllowList: async () => new Set(), assertAuditOpeningsExpected: async () => {},
    ziffleDeckHashFromCommitment: (value) => /^ziffle:([^:]+):\d+$/.exec(String(value || ""))?.[1] || "",

  };
  const apply = compile(between(validation, "  async function applySequencedActionMessageInner(", "  // Forced reveals") + "return applySequencedActionMessageInner;", context);
  return { apply, paused, resume: () => resume(), multiplayerRef, events,
    state: () => ({ engine, applied, commits, releases, restores, clock: matchClockRef.current }) };
}
const message = { seq: 517, actorIndex: 0, command: { type: "pass_priority" } };
for (const verified of [false, true]) for (const phase of ["validate", "engine"]) {
  test(`dispute during ${verified ? "verified" : "trusted"} ${phase} cancels received action transaction and preserves action 516`, async () => {
    const h = actionHarness({ pauseAt: phase, verified });
    const applying = h.apply(message);
    await h.paused;
    h.multiplayerRef.current = disputed();
    h.resume();
    await applying;
    assert.equal(h.state().engine, 516);
    assert.equal(h.state().commits, 0);
    assert.equal(h.state().applied, phase === "engine" ? 1 : 0);
    assert.equal(h.state().restores, 1);
    assert.equal(h.state().releases, 1);
    assert.deepEqual(h.state().clock, acceptedClock());
    assert.equal(h.multiplayerRef.current.lastAppliedSequence, 516);
    assert.match(h.events.find(([name]) => name === "apply_action:failed")?.[1]?.error || "", /match disputed/);
  });
}
test("already disputed actions never enter validation or engine; healthy action still commits", async () => {
  const h = actionHarness();
  h.multiplayerRef.current = disputed();
  await assert.rejects(h.apply(message), /match disputed/);
  assert.equal(h.state().applied, 0);
  for (const verified of [false, true]) {
    const healthy = actionHarness({ verified });
    await healthy.apply(message);
    assert.equal(healthy.state().engine, 517);
    assert.equal(healthy.state().commits, 1);
    assert.equal(healthy.state().restores, 0);
  }
});

test("local submission checks suspension after asynchronous work and uses its rollback rather than accepting a clock", async () => {
  const multiplayerRef = { current: { mode: "in_match", matchStarted: true, lastAppliedSequence: 516 } };
  const prologue = between(lobby, "      const assertSubmissionActive =", "      let session = multiplayerRef.current;");
  const run = compile(prologue + "return runSubmissionPhase;", { ...helpers, multiplayerRef,
    timePeerSyncPhase: async (_phase, _metadata, callback) => callback(),
  });
  let error;
  await assert.rejects(run("apply", {}, async () => {
    multiplayerRef.current = disputed();
    return { sequence: 517 };
  }), (failure) => { error = failure; return failure.code === "MATCH_DISPUTED"; });
  let restored = 0, cancelled = 0;
  const submit = between(lobby, lobby.includes("  const submitVerifiedMultiplayerCommand =")
    ? "  const submitVerifiedMultiplayerCommand =" : "  const submitMultiplayerCommand =",
  "  const submitMultiplayerAddCardCheat");
  const catchBody = between(submit, "        stopLocalActionIntentProgress();\n        clearLocalActionWait();\n        let restoredLocalSubmissionSnapshot", "        const protocolTimeoutClaim =");
  await compile("return (async () => {" + catchBody + "})();", { ...helpers,
    multiplayerRef, err: error, localSubmissionSnapshot: { lastAppliedSequence: 516 },
    localSubmissionCommitted: false, stagedMatchClockRuntime: acceptedClock(),
    signedActionIntent: { seq: 517 }, command: message.command, stopLocalActionIntentProgress() {}, clearLocalActionWait() {},
    restoreSequencedActionValidationSnapshotIfCurrent: async () => { restored += 1; return true; },
    updateMultiplayer: (fn) => { multiplayerRef.current = fn(multiplayerRef.current); },
    toErrorMessage: (failure) => failure.message, recordPeerSyncPerf() {}, summarizePeerCommand: (value) => value,
    broadcastActionIntentCancel: () => { cancelled += 1; }, setStatus() {},
  });
  assert.equal(restored, 1);
  assert.equal(cancelled, 1);
  assert.equal(multiplayerRef.current.lastAppliedSequence, 516);
});

test("transaction rollback preserves a dispute recorded after its savepoint", async () => {
  const multiplayerRef = { current: disputed() };
  const transcript = { current: { disputes: [{ reason: "signed fork evidence" }], actions: [] } };
  const clock = { current: { ...acceptedClock(), clockHash: "uncommitted", lastSequence: 517 } };
  const state = { current: {} };
  const context = { ...helpers, multiplayerRef, liveAuditTranscriptRef: transcript,
    gameRef: { current: { restoreRuntimeSavepoint: async () => {}, uiState: async () => ({ sequence: 516 }) } },
    resolveLocalPlayerIndex: () => 0, actionHistoryRef: { current: [] }, restoreActionCursor: () => [],
    matchStartPayloadRef: { current: {} }, auditStateHashRef: { current: "uncommitted" },
    initialPublicCheckpointHashRef: { current: "" }, matchClockConfigRef: { current: {} }, matchClockRef: clock,
    cloneMultiplayerPayload: structuredClone, ziffleHandRevealKeyRef: { current: "" },
    ziffleHandRevealQuickKeyRef: { current: "" }, restoreSequencedActionCryptoRefs() {}, stateRef: state,
    setState() {}, publishMatchClockSnapshot() {}, runtimeMatchClockSnapshot: () => clock.current,
    updateMultiplayer: (fn) => { multiplayerRef.current = fn(multiplayerRef.current); },
  };
  const restore = compile(between(crypto, "  async function restoreSequencedActionValidationSnapshot(", "\n\n  return {") + "return restoreSequencedActionValidationSnapshot;", context);
  await restore({ game: context.gameRef.current, runtimeHandle: 9, actionHistoryCursor: {}, liveAuditTranscript: { actions: null },
    matchStartPayload: {}, auditStateHash: "accepted-516", matchClockConfig: {}, matchClock: acceptedClock(),
    ziffleHandRevealKey: "", lastAppliedSequence: 516 });
  assert.deepEqual(transcript.current.disputes, [{ reason: "signed fork evidence" }]);
  assert.deepEqual(clock.current, acceptedClock());
  assert.equal(multiplayerRef.current.mode, "disputed");
  assert.equal(multiplayerRef.current.matchStarted, false);
  assert.equal(multiplayerRef.current.lastAppliedSequence, 516);
});

function clockDisputeHarness() {
  const policy = acceptedClock().policy;
  const accepted = { seq: 516, audit: { clock: { seq: 516, clockHash: "verified-clock-516", policy,
    remainingMsByPlayer: acceptedClock().baseRemainingMsByPlayer } } };
  const prospective = { seq: 517, clockHash: "prospective-clock-517", policy,
    remainingMsByPlayer: [880000, 910000] };
  const multiplayerRef = { current: { mode: "in_match", matchStarted: true,
    role: "host", lastAppliedSequence: 516 } };
  const matchClockRef = { current: acceptedClock() };
  const actions = { current: [accepted] };
  let resumePersistence;
  let persistenceEntered;
  const waitingPersistence = new Promise((resolve) => { persistenceEntered = resolve; });
  const context = { ...helpers, multiplayerRef, matchClockRef, actionHistoryRef: actions,
    matchClockConfigRef: { current: policy }, stateRef: { current: {} },
    INITIAL_MATCH_CLOCK_HASH: "zero", normalizeMatchClockPolicy: (value) => value,
    normalizeMatchClockRemaining: (values, count, initial) => values?.length ? [...values] : Array(count).fill(initial),
    cloneMultiplayerPayload: structuredClone,
    runtimeMatchClockSnapshot: () => structuredClone(matchClockRef.current),
    actionTimerSnapshotFromMatchClock: (value) => value,
    updateMultiplayer: (fn) => { multiplayerRef.current = fn(multiplayerRef.current); },
    publishMatchClockSnapshot: (value) => { multiplayerRef.current.matchClock = value; return value; },
    playerCountForClock: () => 2, matchClockActivePlayerFromState: () => 1, nowMonotonicMs: () => 1000,
    recordDiagnosticEvent() {}, recordPeerSyncPerf() {}, pendingSequencedActionsRef: { current: new Map() },
    ignoreAndClearAllPendingActionIntents() {}, liveAuditTranscriptRef: { current: { actions: [accepted] } },
    emitSyncFailureNotice() {}, timeoutClaimInFlightRef: { current: "" }, setStatus() {},
    acceptedActionEntryForMessage: (value) => structuredClone(value), canPersistMatch: () => true,
    matchStartPayloadRef: { current: {} },
    appendRelayAction: async () => { persistenceEntered(); await new Promise((resolve, reject) => {
      resumePersistence = (success) => success ? resolve() : reject(new Error("Durable append failed"));
    }); },
    markActionStage() {}, clearPendingActionIntent() {}, currentAuditMatchId: () => "match",
    captureLocalRuntimeRecoveryIfDue: async () => {},
    auditStateHashRef: { current: "hash516" },
    gameRef: { current: { restoreRuntimeSavepoint: async () => {}, uiState: async () => ({ sequence: 516 }) } },
    resolveLocalPlayerIndex: () => 0, restoreActionCursor: (value) => value,
    initialPublicCheckpointHashRef: { current: "" },
    ziffleHandRevealKeyRef: { current: "" }, ziffleHandRevealQuickKeyRef: { current: "" },
    restoreSequencedActionCryptoRefs() {}, setState() {},
  };
  const declarations = [
    between(crypto, "  function commitMatchClockAudit(", "  function restoreMatchClockRuntimeFromActionTranscript("),
    between(crypto, "  function updateMatchClockForState(", "  function currentMatchClockSnapshot("),
    between(crypto, "  function frozenAcceptedMatchClockRuntime(", "  async function handleHistoricalSequencedAction("),
    between(crypto, "  async function appendAppliedSequencedAction(", "  async function persistRelayCheckpoint("),
    between(crypto, "  async function restoreSequencedActionValidationSnapshot(", "\n\n  return {"),
  ].join("\n");
  const api = compile(declarations + "return { stageLocalMatchClockAudit, commitMatchClockAudit, markMatchDisputed, appendAppliedSequencedAction, restoreSequencedActionValidationSnapshot, restoreMatchClockRuntime };", context);
  return { ...api, multiplayerRef, matchClockRef, actions, prospective, waitingPersistence,
    resumePersistence: (success) => resumePersistence(success),
    snapshot: { game: context.gameRef.current, runtimeHandle: 1, actionHistoryCursor: [accepted], liveAuditTranscript: { actions: null },
      matchStartPayload: {}, auditStateHash: "hash516", matchClockConfig: policy,
      matchClock: acceptedClock(), lastAppliedSequence: 516 },
  };
}
const frozenAcceptedClock = () => ({ ...acceptedClock(), activePlayerIndex: null, epochStartedAtMs: null });

test("dispute immediately replaces a staged prospective clock with the accepted prefix and rollback keeps it frozen", async () => {
  const h = clockDisputeHarness();
  const prior = h.stageLocalMatchClockAudit(h.prospective);
  assert.equal(h.matchClockRef.current.lastSequence, 517);
  h.markMatchDisputed(disputed().matchDisputed.reason);
  assert.deepEqual(h.matchClockRef.current, frozenAcceptedClock());
  assert.equal(h.multiplayerRef.current.matchClock.clockHash, "verified-clock-516");
  await h.restoreSequencedActionValidationSnapshot(h.snapshot);
  assert.deepEqual(h.matchClockRef.current, frozenAcceptedClock());
  h.restoreMatchClockRuntime(prior, {});
  assert.deepEqual(h.matchClockRef.current, frozenAcceptedClock());
});

for (const durableSuccess of [false, true]) {
  test(`dispute during durable acceptance freezes the accepted prefix; ${durableSuccess ? "successful journal append advances it atomically" : "failed journal append rolls back"}`, async () => {
    const h = clockDisputeHarness();
    h.commitMatchClockAudit(h.prospective, {});
    const pending = h.appendAppliedSequencedAction({ seq: 517, audit: { clock: h.prospective } });
    // Attach the rejection observer before resolving the fake persistence I/O.
    const settled = pending.then(() => null, (error) => error);
    await h.waitingPersistence;
    h.markMatchDisputed(disputed().matchDisputed.reason);
    assert.deepEqual(h.matchClockRef.current, frozenAcceptedClock());
    assert.equal(h.actions.current.at(-1).seq, 516);
    h.resumePersistence(durableSuccess);
    const failure = await settled;
    if (durableSuccess) {
      assert.equal(failure, null);
      assert.equal(h.actions.current.at(-1).seq, 517);
      assert.equal(h.multiplayerRef.current.lastAppliedSequence, 517);
      assert.equal(h.matchClockRef.current.clockHash, h.prospective.clockHash);
      assert.equal(h.multiplayerRef.current.matchDisputed.acceptedClockRuntime.lastSequence, 517);
    } else {
      assert.match(failure.message, /Durable append failed/);
      await h.restoreSequencedActionValidationSnapshot(h.snapshot);
      assert.deepEqual(h.matchClockRef.current, frozenAcceptedClock());
      assert.equal(h.actions.current.at(-1).seq, 516);
    }
    assert.equal(h.matchClockRef.current.activePlayerIndex, null);
    assert.equal(h.matchClockRef.current.epochStartedAtMs, null);
    assert.equal(h.multiplayerRef.current.matchDisputed.reason, disputed().matchDisputed.reason);
  });
}

test("only a dispute at the true accepted genesis uses the initial clock hash", () => {
  const h = clockDisputeHarness();
  h.multiplayerRef.current.lastAppliedSequence = 0;
  h.actions.current = [];
  h.stageLocalMatchClockAudit({ ...h.prospective, seq: 1 });
  h.markMatchDisputed("Timeout before the first accepted action");
  assert.equal(h.matchClockRef.current.clockHash, "zero");
  assert.equal(h.matchClockRef.current.lastSequence, 0);
  assert.deepEqual(h.matchClockRef.current.baseRemainingMsByPlayer, [1800000, 1800000]);
});
