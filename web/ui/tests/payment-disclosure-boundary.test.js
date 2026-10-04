// UNVALIDATED. Execute the actual connection-layer functions with deterministic
// transport/engine adapters; no React mount or cryptographic bypass is shipped.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { assertPaymentDisclosureAuthority, createPaymentDisclosureJournal } from '../src/lib/payment-disclosure-journal.js';

const source = readFileSync(new URL('../src/hooks/peer-lobby/connections.js', import.meta.url), 'utf8');
const declarations = source.slice(source.indexOf('  const paymentDisclosures ='), source.indexOf('  const { actionIntentOpeningPreviewKeysRef'));
const remember = source.slice(source.indexOf('  async function rememberPendingActionIntent('), source.indexOf('  async function refreshPendingActionIntentEvidenceForAction('));
const copy = value => value == null ? value : JSON.parse(JSON.stringify(value));
const key = value => [value.matchId, value.seq, value.actorIndex].join(':');
const intent = (overrides = {}) => ({ matchId: 'match-A', seq: 7, actorIndex: 0,
  prevStateHash: 'accepted-6', attemptId: 'original-attempt', signature: 'valid-intent',
  command: { type: 'select_objects', object_ids: [41] }, ...overrides });
const opening = { owner: 0, objectId: 41, card: 'Disclosed hand card' };

function harness() {
  const stored = new Map();
  const calls = [];
  const pending = new Map();
  const state = { decision: { player: 0 } };
  const base = { actionHistoryRef: { current: [{ seq: 6 }] }, auditStateHashRef: { current: 'accepted-6' } };
  const multiplayerRef = { current: { lastAppliedSequence: 6 } };
  const gameRef = { current: {
    uiState: async () => state,
    getPaymentDisclosureForCommand: async command => { calls.push(['classify', copy(command)]); return { required: true, active: false, objects: [41] }; },
    retainPaymentDisclosure: async command => { calls.push(['retain', copy(command)]); return state; },
  } };
  const servicesRef = { current: {
    createSequencedActionValidationSnapshot: async () => ({ release: async () => calls.push(['release']) }),
    verifyAuditOpeningsAgainstManifests: async (openings, options) => calls.push(['verify-openings', copy(openings), copy(options)]),
    revealAuditOpenings: async openings => calls.push(['open', copy(openings)]),
    remapCommandForLocalHiddenOpening: async command => copy(command),
    verifySequencedActionAudit: async () => calls.push(['verify-audit']),
    fairRandomRevealLockConflict: () => false,
  } };
  const context = {
    assertPaymentDisclosureAuthority, createPaymentDisclosureJournal, base, gameRef, servicesRef, multiplayerRef,
    pendingActionIntentsRef: { current: pending }, matchStartPayloadRef: { current: { matchId: 'match-A' } },
    getPeerSessionStorage: () => ({ getItem: name => stored.get(name) ?? null, setItem: (name, value) => stored.set(name, value) }),
    isForfeitCommand: command => command?.type === 'forfeit_player',
    isNonDispatchSyncCommand: command => ['forfeit_player', 'cancel_decision'].includes(command?.type),
    cloneMultiplayerPayload: copy, actionIntentKey: key, currentAuditMatchId: () => 'match-A',
    recordPeerSyncPerf: () => {}, toErrorMessage: String,
    verifySignedActionIntent: async supplied => { calls.push(['verify-intent']); if (supplied.signature !== 'valid-intent') throw new Error('bad signature'); return supplied; },
    actionIntentFingerprint: supplied => JSON.stringify(supplied), protocolActionIntentInactiveReason: () => '',
    matchingAppliedActionForIntent: () => false, shouldReplacePendingActionIntentEvidence: () => true,
    rememberActionIntentObservation: () => {},
    schedulePendingActionIntentTimeout: (_key, record) => calls.push(['timer', record.firstObservedAtMs]),
    observedMatchClockElapsedForIntent: async () => 900,
  };
  const api = new Function(...Object.keys(context), declarations + '\n' + remember + `\nreturn {
    paymentDisclosureForCommand, pinVerifiedPaymentEnvelope, restorePaymentDisclosureAtHead,
    pinPaymentDisclosureIntent, assertPaymentDisclosureIntent, paymentDisclosures, rememberPendingActionIntent,
  };`)(...Object.values(context));
  servicesRef.current.restoreSequencedActionValidationSnapshot = async () => {
    calls.push(['restore-prefix']);
    await api.restorePaymentDisclosureAtHead({ sequence: 7, prevStateHash: 'accepted-6' });
  };
  return { api, calls, pending, state, base, servicesRef };
}

test('actual metadata route accepts non-dispatch Cancel without calling native UiCommand decoding', async () => {
  const { api, calls } = harness();
  assert.equal((await api.paymentDisclosureForCommand({ type: 'cancel_decision' })).required, false);
  assert.equal((await api.paymentDisclosureForCommand({ type: 'forfeit_player', player: 0 })).required, false);
  assert.equal(calls.length, 0);
  await api.paymentDisclosureForCommand(intent().command);
  assert.equal(calls.filter(call => call[0] === 'classify').length, 1);
  assert.equal(api.paymentDisclosures.entries('match-A').length, 0, 'read-only metadata never pins a payment');
});

test('actual incoming path rejects wrong seat before proof hydration or durable lock, then permits rightful payer', async () => {
  const { api, calls } = harness();
  const wrong = intent({ actorIndex: 1 });
  await assert.rejects(api.pinVerifiedPaymentEnvelope(wrong, [opening], { actionIntent: wrong }), /decision player/);
  assert.deepEqual(calls, []);
  assert.equal(api.paymentDisclosures.entries('match-A').length, 0);
  await api.pinVerifiedPaymentEnvelope(intent(), [opening], { actionIntent: intent() });
  assert.equal(api.paymentDisclosures.entries('match-A').length, 1);
  assert.ok(calls.findIndex(call => call[0] === 'verify-openings') < calls.findIndex(call => call[0] === 'open'));
  assert.ok(calls.some(call => call[0] === 'retain'), 'rollback preserves the same published command');
  assert.throws(() => api.assertPaymentDisclosureIntent({ ...intent(), command: { type: 'cancel_decision' } }), /exact command/);
});

test('actual recovery restores original intent timing after pending records and timers were cleared', async () => {
  const { api, calls, pending, state } = harness();
  const original = intent();
  const evidence = { requestId: 'original-request', requestedAtMs: 1100, responseTimeoutMs: 250,
    requestPayload: { actionIntent: original } };
  pending.set(key(original), { intent: original, fingerprint: JSON.stringify(original), evidence,
    firstObservedAtMs: 1000, observedElapsedAtIntentMs: 700 });
  await api.pinVerifiedPaymentEnvelope(original, [opening], { actionIntent: original });
  pending.clear();
  calls.length = 0;
  await api.restorePaymentDisclosureAtHead({ sequence: 7, prevStateHash: 'accepted-6' });
  const restored = pending.get(key(original));
  assert.equal(restored.firstObservedAtMs, 1000);
  assert.equal(restored.observedElapsedAtIntentMs, 900);
  assert.deepEqual(restored.evidence, evidence);
  assert.deepEqual(restored.intent, original);
  assert.ok(calls.some(call => call[0] === 'timer' && call[1] === 1000));
  state.decision.player = 1;
  calls.length = 0;
  await assert.rejects(api.restorePaymentDisclosureAtHead({ sequence: 7, prevStateHash: 'accepted-6' }), /decision player/);
  assert.equal(calls.length, 0, 'recovery cannot hydrate another decision owner');
});

test('actual incoming path never records an opening whose proof verification fails', async () => {
  const { api, servicesRef } = harness();
  servicesRef.current.verifyAuditOpeningsAgainstManifests = async () => { throw new Error('bad opening proof'); };
  await assert.rejects(api.pinVerifiedPaymentEnvelope(intent(), [opening], { actionIntent: intent() }), /bad opening proof/);
  assert.equal(api.paymentDisclosures.entries('match-A').length, 0);
});


test('actual pending map and journal reject a new attempt without changing the retained retry', async () => {
  const { api, pending } = harness();
  const original = intent();
  await api.pinVerifiedPaymentEnvelope(original, [opening], { actionIntent: original });
  const savedRecord = copy(pending.get(key(original)));
  const savedJournal = api.paymentDisclosures.lookup(original);
  await assert.rejects(api.rememberPendingActionIntent({ ...original, attemptId: 'different-attempt' }), /original signed attempt/);
  await assert.rejects(api.rememberPendingActionIntent({ ...original, preActionPublicCheckpointHash: 'different-checkpoint' }), /original signed attempt/);
  assert.deepEqual(pending.get(key(original)), savedRecord);
  assert.deepEqual(api.paymentDisclosures.lookup(original), savedJournal);
});
