// UNVALIDATED: authored under the source-only campaign workflow.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createPaymentDisclosureJournal } from '../src/lib/payment-disclosure-journal.js';

function memoryStorage() {
  const values = new Map();
  return { getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) };
}
const intent = (changes = {}) => ({ matchId: 'match-A', seq: 7, actorIndex: 0, prevStateHash: 'accepted-6',
  command: { type: 'select_objects', object_ids: [41], source: 10, x: 3, targets: [20] }, ...changes });
const opening = { owner: 0, objectId: 41, cardName: 'Payment card', commitment: 'committed-41' };

test('published choice survives recovery and accepts only the same payer, source, X, targets and incarnation', () => {
  const storage = memoryStorage();
  const journal = createPaymentDisclosureJournal(storage);
  const original = intent();
  journal.pin(original, { openings: [opening], evidence: { actionIntent: { ...original, signature: 'signed' } } });
  const recovered = createPaymentDisclosureJournal(storage);
  assert.deepEqual(recovered.assertCompatible(intent()).command, original.command);
  for (const command of [
    { ...original.command, object_ids: [42] }, { ...original.command, source: 11 },
    { ...original.command, x: 4 }, { ...original.command, targets: [21] },
  ]) assert.throws(() => recovered.assertCompatible(intent({ command })), /exact command/);
  assert.throws(() => recovered.assertCompatible(intent({ actorIndex: 1 })), /actor or accepted prefix/);
  assert.throws(() => recovered.assertCompatible(intent({ prevStateHash: 'other-prefix' })), /actor or accepted prefix/);
  assert.equal(recovered.lookup(intent({ matchId: 'another-match' })), null);
});

test('retry merges openings, does not forget earlier publication, and returned data cannot edit its commitment', () => {
  const journal = createPaymentDisclosureJournal(memoryStorage());
  journal.pin(intent(), { openings: [opening], evidence: { actionIntent: { ...intent(), signature: 'signed' } } });
  const later = { ...opening, objectId: 42, commitment: 'committed-42' };
  const retained = journal.pin(intent(), { openings: [later] });
  retained.command.object_ids[0] = 100;
  retained.openings.length = 0;
  assert.deepEqual(journal.lookup(intent()).openings, [opening, later]);
  assert.equal(journal.lookup(intent()).command.object_ids[0], 41);
  assert.equal(journal.lookup(intent()).evidence.actionIntent.signature, 'signed');
  journal.pin(intent(), { openings: [opening] });
  assert.equal(journal.lookup(intent()).openings.length, 2);
});

test('unaccepted failure or cancellation cannot clear a pin; only an accepted prefix retires it', () => {
  const journal = createPaymentDisclosureJournal(memoryStorage());
  journal.pin(intent(), { openings: [opening] });
  journal.accepted('match-A', 6);
  assert.ok(journal.lookup(intent()));
  assert.equal(typeof journal.cancel, 'undefined');
  journal.pin(intent({ seq: 8, prevStateHash: 'accepted-7' }), { openings: [opening] });
  journal.accepted('match-A', 7);
  assert.equal(journal.lookup(intent()), null);
  assert.ok(journal.lookup(intent({ seq: 8, prevStateHash: 'accepted-7' })));
});

test('storage refusal stops publication before its callback can run', () => {
  let published = false;
  for (const storage of [null, { getItem: () => null, setItem: () => {} },
    { getItem: () => null, setItem: () => { throw new Error('quota'); } }]) {
    const journal = createPaymentDisclosureJournal(storage);
    assert.throws(() => { journal.pin(intent(), { openings: [opening] }); published = true; });
    assert.equal(published, false);
  }
});

// Wiring controls supplement behavioral tests: every outgoing publication path
// must establish the same journal boundary, while speculative previews do not.
const source = path => readFileSync(new URL('../src/' + path, import.meta.url), 'utf8');
test('payment routing and recovery preserve the opening/proof validation boundary', () => {
  const optimistic = source('hooks/peer-lobby/optimistic-state.js');
  assert.ok(optimistic.indexOf('getPaymentDisclosureForCommand(command)') < optimistic.indexOf('const publicClaims = []'));
  assert.match(optimistic, /if \(paymentDisclosure\) return;/);
  const connections = source('hooks/peer-lobby/connections.js');
  const recovery = connections.slice(connections.indexOf('async function restorePaymentDisclosureAtHead'), connections.indexOf('async function paymentDisclosureForCommand'));
  assert.ok(recovery.indexOf('verifySignedActionIntent') < recovery.indexOf('retainPaymentDisclosure(localCommand)'));
  assert.ok(recovery.indexOf('verifyAuditOpeningsAgainstManifests') < recovery.indexOf('revealAuditOpenings'));
  const crypto = source('hooks/peer-lobby/crypto-resync.js');
  assert.match(crypto, /pinVerifiedPaymentEnvelope\(actionIntent, message.openings/);
  assert.ok(crypto.indexOf('pinPaymentDisclosureIntent(actionIntent') < crypto.indexOf('const requestPayload = {', crypto.indexOf('pinPaymentDisclosureIntent(actionIntent')));
  assert.match(crypto, /restorePaymentDisclosureAtHead/);
  assert.match(source('hooks/peer-lobby/messaging.js'), /restorePaymentDisclosureAtHead/);
  const bridge = source('hooks/useWasmGame.js');
  assert.match(bridge, /"getPaymentDisclosureForCommand"/);
  assert.match(bridge, /"retainPaymentDisclosure"/);
  const submission = source('hooks/usePeerLobby.js');
  assert.match(submission, /extraPayload = openingPreparationProgress\(extraPayload/);
  assert.match(submission, /pinPaymentDisclosureIntent\(signedActionIntent/);
});

test('wrong-seat or wrong-head disclosure cannot pin a sequence, but the current replacement chooser may', async () => {
  const { assertPaymentDisclosureAuthority } = await import('../src/lib/payment-disclosure-journal.js');
  const journal = createPaymentDisclosureJournal(memoryStorage());
  const head = { matchId: 'match-A', lastAppliedSequence: 6, prevStateHash: 'accepted-6', decisionPlayer: 0 };
  const accept = candidate => { assertPaymentDisclosureAuthority(candidate, head); journal.pin(candidate, { openings: [opening] }); };
  for (const changed of [{ actorIndex: 1 }, { seq: 8 }, { prevStateHash: 'another-head' }, { matchId: 'another-match' }]) {
    assert.throws(() => accept(intent(changed)));
    assert.equal(journal.entries('match-A').length, 0);
  }
  accept(intent());
  assert.ok(journal.lookup(intent()), 'rightful payer remains able to proceed after rejected wrong-seat input');
  assert.doesNotThrow(() => assertPaymentDisclosureAuthority(intent({ actorIndex: 1 }), { ...head, decisionPlayer: 1 }),
    'a replacement decision belongs to its actual chooser, even when the original payer was player zero');
});

test('non-dispatch Cancel bypasses metadata decoding while keeping native and journal guards', () => {
  const connections = source('hooks/peer-lobby/connections.js');
  const metadata = connections.slice(connections.indexOf('async function paymentDisclosureForCommand'), connections.indexOf('  const { actionIntentOpeningPreviewKeysRef'));
  assert.ok(metadata.indexOf('isNonDispatchSyncCommand(command)') < metadata.indexOf('getPaymentDisclosureForCommand(command)'));
  const optimistic = source('hooks/peer-lobby/optimistic-state.js');
  assert.match(optimistic, /const disclosure = isNonDispatchSyncCommand\(command\)\s*\? null : await visibleGame\(\).getPaymentDisclosureForCommand/);
  const validation = source('hooks/peer-lobby/validation.js');
  assert.ok(validation.indexOf('Sequenced action actor is not the current decision player') < validation.indexOf('pinVerifiedPaymentEnvelope'));
  const incoming = connections.slice(connections.indexOf('async function pinVerifiedPaymentEnvelope'), connections.indexOf('async function restorePaymentDisclosureAtHead'));
  assert.ok(incoming.indexOf('validatePaymentDisclosureAuthority(intent)') < incoming.indexOf('pinPaymentDisclosureIntent(intent'));
});

test('same-head recovery retains original hard deadline, observed clock, signed intent and request evidence', () => {
  const storage = memoryStorage();
  const journal = createPaymentDisclosureJournal(storage);
  const originalIntent = { ...intent(), attemptId: 'original-attempt', signature: 'signed-original' };
  const timing = { intent: originalIntent, firstObservedAtMs: 1000, observedElapsedAtIntentMs: 700,
    evidence: { requestId: 'first-request', requestedAtMs: 1100, responseTimeoutMs: 250,
      requestPayload: { actionIntent: originalIntent } }, timeoutConfirmation: { dueAtMs: 1400, notBeforeMs: 1450 } };
  journal.pin(originalIntent, { openings: [opening], timing });
  const recovered = createPaymentDisclosureJournal(storage);
  assert.deepEqual(recovered.lookup(intent()).timing, timing);
  recovered.pin(intent(), { timing: { ...timing, firstObservedAtMs: 9000, observedElapsedAtIntentMs: 300 } });
  recovered.pin(intent(), { timing: { firstObservedAtMs: 9500, observedElapsedAtIntentMs: null } });
  const retained = recovered.lookup(intent()).timing;
  assert.equal(retained.firstObservedAtMs, 1000, 'reload cannot buy a new hard deadline');
  assert.equal(retained.observedElapsedAtIntentMs, 700, 'already observed thinking time cannot disappear');
  assert.deepEqual(retained.intent, originalIntent);
  assert.deepEqual(retained.evidence, timing.evidence);
  assert.deepEqual(retained.timeoutConfirmation, timing.timeoutConfirmation);
  const connections = source('hooks/peer-lobby/connections.js');
  assert.match(connections, /firstObservedAtMs: retainedTiming\?\.firstObservedAtMs \|\| Date.now\(\)/);
  assert.match(connections, /rememberPendingActionIntent\(pendingIntent, retained.timing\?\.evidence/);
  assert.match(connections, /function schedulePendingActionIntentTimeout\(key, record\) \{\s*if \(!key \|\| !record\?\.intent\) return;\s*persistPendingPaymentTiming\(record\)/);
});

test('signed payment attempt and pre-action checkpoint stay immutable while bound material may merge', () => {
  const journal = createPaymentDisclosureJournal(memoryStorage());
  const original = { ...intent(), attemptId: 'one-attempt', preActionPublicCheckpointHash: 'public-before', signature: 'signature-one' };
  journal.pin(original, { openings: [opening], evidence: { actionIntent: original },
    timing: { firstObservedAtMs: 1000, intent: original } });
  const before = journal.lookup(original);
  for (const changed of [{ attemptId: 'different-attempt' }, { preActionPublicCheckpointHash: 'different-public-before' }]) {
    const replacement = { ...original, ...changed };
    assert.throws(() => journal.assertCompatible(replacement), /original signed attempt/);
    assert.throws(() => journal.pin(intent(), { evidence: { actionIntent: replacement } }), /original signed attempt/);
    assert.throws(() => journal.pin(intent(), { timing: { firstObservedAtMs: 2000, intent: replacement } }), /original signed attempt/);
    assert.deepEqual(journal.lookup(original), before, 'rejected material must not mutate either canonical or timing intent');
  }
  journal.pin(original, { openings: [{ ...opening, objectId: 42 }], evidence: { actionIntent: { ...original, signature: 'another-valid-signature' } } });
  const after = journal.lookup(original);
  assert.equal(after.openings.length, 2);
  assert.deepEqual(after.signedIntent, original);
  assert.deepEqual(after.evidence.actionIntent, original);
  assert.deepEqual(after.timing.intent, original);
});
