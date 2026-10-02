import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const source = readFileSync(new URL('../src/hooks/peer-lobby/connections.js', import.meta.url), 'utf8');
function implementation(name) {
  const start = source.search(new RegExp(`^  (?:async )?function ${name}\\(`, 'm'));
  assert.ok(start >= 0, `${name} is present`);
  const end = source.indexOf('\n  }\n', start);
  assert.ok(end > start);
  return source.slice(start, end + 4);
}

function harness() {
  let now = 1000000;
  let nextTimer = 0;
  const timers = new Map(), observations = [], claims = [], events = [];
  const key = 'match:517:0';
  const intent = { matchId: 'match', seq: 517, actorIndex: 0, command: { type: 'mana_payment' } };
  const context = {
    Date: { now: () => now }, console, structuredClone,
    window: {
      setTimeout: (fn, delay) => { const id = ++nextTimer; timers.set(id, { fn, delay, due: now + delay }); return id; },
      clearTimeout: id => timers.delete(id),
    },
    document: { hidden: false, visibilityState: 'visible' },
    PROTOCOL_VERSION: 15, PROTOCOL_RESPONSE_TIMEOUT_MS: 120000, MAX_PENDING_ACTION_INTENT_MS: 520000,
    MATCH_CLOCK_CLAIM_SKEW_MS: 2000, ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD: 4000,
    pendingActionIntentsRef: { current: new Map() }, pendingActionIntentTimeoutsRef: { current: new Map() },
    actionIntentOpeningPreviewKeysRef: { current: new Map() },
    multiplayerRef: { current: { mode: 'in_match', matchStarted: true, lastAppliedSequence: 516, localPlayerIndex: 1 } },
    gameRef: { current: { uiState: () => new Promise(resolve => observations.push(resolve)) } },
    stateRef: { current: {} },
    currentAuditMatchId: () => 'match', actionIntentKey: value => `${value.matchId}:${value.seq}:${value.actorIndex}`,
    actionIntentFingerprint: value => JSON.stringify(value), signedActionIntentPayload: value => value,
    verifySignedActionIntent: async value => { if (value.signature === 'invalid') throw Error('invalid signature'); return value; },
    protocolActionIntentInactiveReason: () => context.multiplayerRef.current.matchStarted ? '' : 'match_disputed',
    matchingAppliedActionForIntent: value => value.seq <= context.multiplayerRef.current.lastAppliedSequence ? {} : null,
    cloneMultiplayerPayload: structuredClone, clearPeerWaitForActionIntent() {},
    updateMatchClockForState: () => { events.push('clock observation'); return { enabled: false }; },
    nowMonotonicMs: () => now,
    recordPeerSyncPerf: (kind, metadata) => events.push({ kind, metadata }),
    normalizePlayerIndex: value => value, canonicalMultiplayerPayload: JSON.stringify,
    sha256Hex: async () => 'hash', resolveLocalPlayerIndex: () => 1,
    playerForProtocolResponseTimeout: () => ({ peerId: 'host', name: 'Alice' }),
    submitProtocolResponseTimeoutClaim: async claim => claims.push(claim),
    emitSyncFailureNotice() {}, setStatus() {}, toErrorMessage: error => error.message,
    // No fair-random reveal is locked to another intent in these scenarios.
    servicesRef: { current: { fairRandomRevealLockConflict: () => false } },
    // Protocol-wait bookkeeping (timeout voters' local observations).
    protocolWaitObservationsRef: { current: new Map() }, PROTOCOL_WAIT_MAX_OBSERVATIONS: 512,
  };
  vm.createContext(context);
  const names = [
    'clearPendingActionIntent', 'clearAllPendingActionIntents',
    'pendingActionIntentEvidenceTimeoutMs', 'pendingActionIntentEvidenceRequestedAtMs',
    'pendingActionIntentFirstObservedAtMs', 'pendingActionIntentEvidenceDueAtMs',
    'pendingActionIntentHardDueAtMs', 'pendingActionIntentDueAtMs',
    'shouldReplacePendingActionIntentEvidence', 'pendingActionIntentHardTimeoutEvidence',
    'schedulePendingActionIntentTimeout', 'observedMatchClockElapsedForIntent',
    'rememberPendingActionIntent', 'handlePendingActionIntentTimeout',
    'rememberActionIntentObservation', 'pruneProtocolWaitObservations',
  ];
  vm.runInContext(names.map(implementation).join('\n'), context);
  const evidence = timeout => ({ requestType: 'action_intent_progress', requestId: `progress-${timeout}`,
    requestPayload: { phase: 'payload_generation' }, requestPayloadHash: 'hash', responseTimeoutMs: timeout, requestedAtMs: now });
  const remember = (timeout, value = intent) => context.rememberPendingActionIntent(value, evidence(timeout));
  const settle = () => new Promise(resolve => setImmediate(resolve));
  const advance = async ms => {
    now += ms;
    const due = [...timers.entries()].filter(([, timer]) => timer.due <= now);
    for (const [id, timer] of due) { if (!timers.delete(id)) continue; timer.fn(); }
    await settle();
  };
  return { context, intent, key, timers, observations, claims, events, remember, settle, advance,
    record: () => context.pendingActionIntentsRef.current.get(key), setNow: value => { now = value; } };
}

test('overlapping initial progress cannot replace a 120-second allowance with four seconds', async () => {
  const h = harness();
  const long = h.remember(120000), short = h.remember(4000);
  await h.settle();
  assert.equal(h.observations.length, 2);
  h.observations[0]({}); await long;
  assert.equal(h.record().evidence.responseTimeoutMs, 120000);
  h.observations[1]({}); await short;
  assert.equal(h.record().evidence.responseTimeoutMs, 120000);
  await h.advance(19000);
  assert.equal(h.claims.length, 0, 'queued valid work is not disputed after 19 seconds');
});

test('deadline is recorded before a slow clock observation completes', async () => {
  const h = harness();
  const pending = h.remember(120000); await h.settle();
  assert.equal(h.record()?.evidence.responseTimeoutMs, 120000);
  assert.equal(h.timers.size, 1);
  h.observations[0]({}); await pending;
});

for (const transition of ['cancel', 'applied', 'disputed', 'replaced']) {
  test(`late clock observation cannot resurrect an intent after it is ${transition}`, async () => {
    const h = harness();
    const pending = h.remember(4000); await h.settle();
    h.context.clearPendingActionIntent(h.key);
    if (transition === 'applied') h.context.multiplayerRef.current.lastAppliedSequence = 517;
    if (transition === 'disputed') {
      h.context.multiplayerRef.current.mode = 'disputed';
      h.context.multiplayerRef.current.matchStarted = false;
    }
    const replacement = transition === 'replaced' ? { intent: h.intent, evidence: { responseTimeoutMs: 120000 } } : undefined;
    if (replacement) h.context.pendingActionIntentsRef.current.set(h.key, replacement);
    h.observations[0]({}); await pending;
    assert.equal(h.record(), replacement);
    assert.equal(h.timers.size, 0);
    assert.equal(h.events.filter(event => event === 'clock observation').length, 0);
  });
}

test('conflicting and unauthenticated intent messages cannot extend the current deadline', async () => {
  const h = harness(); const pending = h.remember(4000); await h.settle();
  h.observations[0]({}); await pending;
  await assert.rejects(h.remember(120000, { ...h.intent, signature: 'invalid' }), /invalid signature/);
  await assert.rejects(h.remember(120000, { ...h.intent, command: { type: 'different' } }), /conflicting signed action intent/);
  assert.equal(h.record().evidence.responseTimeoutMs, 4000);
});

async function readyIntent(h, timeout = 4000) {
  const pending = h.remember(timeout); await h.settle();
  h.observations.at(-1)({}); await pending;
}

test('late timer allows queued authenticated progress to extend the deadline before claiming', async () => {
  const h = harness(); await readyIntent(h);
  await h.advance(19000);
  assert.equal(h.claims.length, 0);
  assert.equal([...h.timers.values()][0].delay, 6000);
  await readyIntent(h, 120000);
  await h.advance(6000);
  assert.equal(h.claims.length, 0);
  assert.equal(h.record().evidence.responseTimeoutMs, 120000);
});

test('genuine silence still times out after a bounded catch-up observation', async () => {
  for (const [delay, recovery] of [[6000, 2000], [19000, 6000]]) {
    const h = harness(); await readyIntent(h);
    await h.advance(delay);
    assert.equal(h.claims.length, 0);
    await h.advance(recovery);
    assert.equal(h.claims.length, 1);
    assert.equal(h.claims[0].responseTimeoutMs, 4000, 'catch-up does not rewrite signed timeout evidence');
    assert.equal(h.claims[0].basisSequence, 516);
  }
});

test('another local suspension requires a normally scheduled observation before blaming the peer', async () => {
  const h = harness(); await readyIntent(h);
  await h.advance(19000);
  await h.advance(19000);
  assert.equal(h.claims.length, 0);
  await h.advance(6000);
  assert.equal(h.claims.length, 1);
});

test('a queued cancelled callback cannot consume a replacement timer', async () => {
  const h = harness(); await readyIntent(h);
  const oldTimer = [...h.timers.values()][0];
  await readyIntent(h, 120000);
  const replacement = [...h.timers.keys()][0];
  h.setNow(1019000); oldTimer.fn(); await h.settle();
  assert.equal(h.context.pendingActionIntentTimeoutsRef.current.get(h.key), replacement);
  assert.equal(h.timers.has(replacement), true);
  assert.equal(h.claims.length, 0);
});

test('progress cannot move the absolute action hard deadline', async () => {
  const h = harness(); await readyIntent(h, 120000);
  for (let i = 0; i < 5; i++) {
    await h.advance(100000);
    await readyIntent(h, 120000);
  }
  await h.advance(22000);
  assert.equal(h.claims.length, 0);
  await h.advance(2000);
  assert.equal(h.claims.length, 1);
  assert.equal(h.claims[0].requestType, 'pending_action_intent');
  assert.equal(h.claims[0].requestedAtMs, 1000000);
});

for (const transition of ['progress', 'clear', 'applied', 'disputed', 'new-match']) {
  test(`timeout hashing cannot claim stale evidence after ${transition}`, async () => {
    const h = harness(); await readyIntent(h);
    h.record().evidence.requestPayloadHash = '';
    let finishHash;
    h.context.sha256Hex = () => new Promise(resolve => { finishHash = resolve; });
    await h.advance(6000); await h.advance(2000);
    assert.equal(typeof finishHash, 'function');
    if (transition === 'progress') await readyIntent(h, 120000);
    if (transition === 'clear') h.context.clearPendingActionIntent(h.key);
    if (transition === 'applied') h.context.multiplayerRef.current.lastAppliedSequence = 517;
    if (transition === 'disputed') h.context.multiplayerRef.current.matchStarted = false;
    if (transition === 'new-match') h.context.currentAuditMatchId = () => 'replacement';
    finishHash('hash'); await h.settle();
    assert.equal(h.claims.length, 0);
    if (transition === 'progress') {
      assert.equal(h.timers.size, 1, 'renewed deadline remains scheduled');
      assert.equal(h.record().evidence.responseTimeoutMs, 120000);
    }
  });
}

test('suspension during timeout hashing gives queued packets a fresh observation window', async () => {
  const h = harness(); await readyIntent(h);
  h.record().evidence.requestPayloadHash = '';
  let finishHash;
  h.context.sha256Hex = () => new Promise(resolve => { finishHash = resolve; });
  await h.advance(6000); await h.advance(2000);
  h.setNow(1027000); finishHash('hash'); await h.settle();
  assert.equal(h.claims.length, 0);
  assert.equal([...h.timers.values()][0].delay, 6000);
  h.context.sha256Hex = async () => 'hash';
  await h.advance(6000);
  assert.equal(h.claims.length, 1);
});

function retryHarness() {
  const h = harness();
  const c = h.context;
  c.ignoredActionIntentKeysRef = { current: new Map() };
  c.IGNORED_ACTION_INTENT_TTL_MS = 600000; c.MAX_IGNORED_ACTION_INTENTS = 256;
  c.actionIntentCancelPayload = (intent, reason) => ({ intent, reason });
  c.importCachedAuditPublicKey = async () => 'key'; c.publicKeyForAuditSigner = () => 'key';
  c.verifyAuditPayload = async (_key, _payload, signature) => signature === 'valid';
  c.markActionIntentObservationCancelled = () => {};
  c.servicesRef.current.cancelOptimisticIntent = async () => {};
  vm.runInContext(['pruneIgnoredActionIntents', 'rememberIgnoredActionIntentKey',
    'ignoredActionIntentReason', 'protocolActionIntentInactiveReason',
    'handleActionIntentCancelMessage'].map(implementation).join('\n'), c);
  return h;
}

test('cancelling one signed attempt permits a fresh attempt at the same sequence', async () => {
  const h = retryHarness();
  const first = { ...h.intent, attemptId: 'first' };
  let pending = h.remember(4000, first); await h.settle(); h.observations.pop()({}); await pending;
  await h.context.handleActionIntentCancelMessage({ actionIntent: first, cancelSignature: 'valid', reason: 'failed' });
  assert.equal(h.record(), undefined);
  assert.equal(h.context.ignoredActionIntentReason(h.key, first), 'failed');
  const retry = { ...first, attemptId: 'retry', command: { type: 'select_objects', object_ids: [] } };
  pending = h.remember(4000, retry); await h.settle(); h.observations.pop()({}); await pending;
  assert.equal(h.record().intent.attemptId, 'retry');
  assert.equal(h.context.ignoredActionIntentReason(h.key, retry), '');
  await h.context.handleActionIntentCancelMessage({ actionIntent: first, cancelSignature: 'valid', reason: 'late cancel' });
  assert.equal(h.record().intent.attemptId, 'retry', 'a delayed old cancel cannot drop the new attempt');
});

test('fresh attempt IDs cannot bypass a command pinned by disclosed material', async () => {
  const h = retryHarness();
  h.context.servicesRef.current.fairRandomRevealLockConflict = value => value.command.type !== 'mana_payment';
  await assert.rejects(h.remember(4000, { ...h.intent, attemptId: 'new', command: { type: 'select_objects' } }),
    /conflicting signed action intent/);
  assert.equal(h.record(), undefined);
});

test('forged cancellation cannot retire an attempt or clear its pending record', async () => {
  const h = retryHarness(); await readyIntent(h);
  await assert.rejects(h.context.handleActionIntentCancelMessage({ actionIntent: h.intent, cancelSignature: 'forged' }),
    /not signed/);
  assert.ok(h.record()); assert.equal(h.context.ignoredActionIntentKeysRef.current.size, 0);
});
