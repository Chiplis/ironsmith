import test from 'node:test';
import assert from 'node:assert/strict';
import {
  acceptedZiffleEpochs, assertZiffleEpochHistory, assertZiffleEpochInputs,
  assertZiffleEpochVerification, buildZiffleInputDeck, ziffleEpochInputCommitments,
  ziffleEpochMaterial, ziffleEpochNode, ziffleInputDeckFields,
} from '../src/lib/ziffle-private-epochs.js';

// These tests cover the application's transcript/game-state binding. Actual
// shuffle and reveal proof validation is separately exercised by WASM tests.
function fixture() {
  const genesis = { owner: 0, deckCount: 8, context: 'match:owner:0:initial', deckHash: 'genesis-a',
    steps: [{ shuffler: 0, deckHex: 'encrypted-root-a', proofHex: 'proof-a' },
      { shuffler: 1, deckHex: 'encrypted-root-b', proofHex: 'proof-b' }] };
  const other = { ...genesis, owner: 1, context: 'match:owner:1:initial', deckHash: 'genesis-b' };
  const inputCommitments = [0, 2, 4, 7].map(position => `ziffle:${genesis.deckHash}:${position}`);
  const first = { owner: 0, deckCount: 4, context: 'match:owner:0:shuffle:1', deckHash: 'first-a',
    inputDeck: buildZiffleInputDeck([genesis], inputCommitments),
    steps: [{ shuffler: 0, deckHex: 'encrypted-first-a', proofHex: 'proof-c' },
      { shuffler: 1, deckHex: 'encrypted-first-b', proofHex: 'proof-d' }] };
  const nextCommitments = [0, 1, 2, 3].map(position => `ziffle:${first.deckHash}:${position}`);
  const second = { owner: 0, deckCount: 4, context: 'match:owner:0:shuffle:2', deckHash: 'second-a',
    inputDeck: buildZiffleInputDeck([genesis, first], nextCommitments),
    steps: [{ shuffler: 0, deckHex: 'encrypted-second-a', proofHex: 'proof-e' },
      { shuffler: 1, deckHex: 'encrypted-second-b', proofHex: 'proof-f' }] };
  return { genesis, other, first, second, inputCommitments, nextCommitments };
}

function assertNoIdentityMapping(value) {
  const forbidden = new Set(['beforeOrder', 'afterOrder', 'before_order', 'after_order', 'authenticatedOrder',
    'objectId', 'object_id', 'stableId', 'stable_id', 'originalSlot', 'originSlot', 'originCommitment', 'cardName', 'reveals', 'tokens']);
  if (value && typeof value === 'object') for (const [key, child] of Object.entries(value)) {
    assert.equal(forbidden.has(key), false, `private shuffle contains identity linkage ${key}`);
    assertNoIdentityMapping(child);
  }
}

test('private shuffle graph carries only ciphertext inputs and encrypted epoch proofs', () => {
  const { genesis, first, inputCommitments } = fixture();
  const decorated = { ...genesis, beforeOrder: [90, 91, 92], afterOrder: [92, 90, 91],
    reveals: [{ originalSlot: 7, cardPosition: 0 }], tokens: [{ secret: 'local-only' }],
    objectId: 90, originSlot: 7, cardName: 'Island' };
  const graph = buildZiffleInputDeck([decorated], [...inputCommitments].reverse());
  assert.deepEqual(graph, first.inputDeck, 'deterministic input order depends only on prior ciphertext positions');
  assertNoIdentityMapping(graph);
  assert.deepEqual(ziffleEpochNode(first), { deckCount: 4, context: first.context, steps: first.steps,
    sources: first.inputDeck.sources });
  const publicFields = ziffleInputDeckFields(first);
  publicFields.inputDeck.sources[0].position = 6;
  assert.equal(first.inputDeck.sources[0].position, 0, 'returned graph cannot mutate accepted history');
});

test('every accepted initial and prior epoch is pinned exactly in order', () => {
  const { genesis, first, second } = fixture();
  assert.doesNotThrow(() => assertZiffleEpochHistory(second, [genesis, first]));
  for (const mutate of [
    proof => { proof.inputDeck.universeCount = 9; },
    proof => { proof.inputDeck.epochs[0].context = 'another-match'; },
    proof => { proof.inputDeck.epochs[0].steps[0].deckHex = 'another-encrypted-root'; },
    proof => { proof.inputDeck.epochs[0].steps[1].proofHex = 'another-proof'; },
    proof => { proof.inputDeck.epochs[1].context += ':fork'; },
    proof => { proof.inputDeck.epochs[1].sources[0].position = 1; },
    proof => { proof.inputDeck.epochs.reverse(); },
    proof => { proof.inputDeck.epochs.pop(); },
    proof => { proof.inputDeck.epochs.push(proof.inputDeck.epochs[0]); },
  ]) {
    const changed = structuredClone(second); mutate(changed);
    assert.throws(() => assertZiffleEpochHistory(changed, [genesis, first]), /accepted signed transcript/);
  }
});

test('new private proofs reject legacy public correspondence maps', () => {
  const { genesis, first } = fixture();
  for (const key of ['beforeOrder', 'afterOrder', 'before_order', 'after_order', 'authenticatedOrder']) {
    assert.throws(() => assertZiffleEpochHistory({ ...first, [key]: [] }, [genesis]), /public object-order mapping/);
  }
});

test('ciphertext sources must be unique, valid back-references and not already consumed', () => {
  const { genesis, first, second } = fixture();
  for (const source of [{ epoch: -1, position: 0 }, { epoch: 2, position: 0 },
    { epoch: 1, position: 4 }, { epoch: 1, position: -1 }, { epoch: 1, position: 1.5 },
    { epoch: '1', position: 0 }, { epoch: 0, position: 0 }]) {
    const bad = structuredClone(second); bad.inputDeck.sources[0] = source;
    assert.throws(() => assertZiffleEpochHistory(bad, [genesis, first]), /invalid or already consumed/);
  }
  const duplicate = structuredClone(second); duplicate.inputDeck.sources[1] = duplicate.inputDeck.sources[0];
  assert.throws(() => assertZiffleEpochHistory(duplicate, [genesis, first]), /invalid or already consumed/);
  const missing = structuredClone(second); missing.inputDeck.sources.pop();
  assert.throws(() => assertZiffleEpochHistory(missing, [genesis, first]), /source count mismatch/);
  assert.throws(() => buildZiffleInputDeck([genesis, first], ['ziffle:genesis-a:0']), /already consumed/);
});

test('a known card excluded from a shuffle can be reinserted without an output mapping', () => {
  const { genesis, first, nextCommitments } = fixture();
  const returnedCard = 'ziffle:genesis-a:1';
  const sources = [...nextCommitments, returnedCard];
  const returned = { owner: 0, context: 'match:return-known-card', deckCount: 5,
    inputDeck: buildZiffleInputDeck([genesis, first], sources) };
  assert.deepEqual(returned.inputDeck.sources[0], { epoch: 0, position: 1 });
  assert.deepEqual(ziffleEpochInputCommitments(returned, [genesis, first]), [returnedCard, ...nextCommitments]);
  assert.doesNotThrow(() => assertZiffleEpochInputs(returned, { inputCommitments: sources }, [genesis, first]));
  assertNoIdentityMapping(returned.inputDeck);
});

test('a valid same-size shuffle set must still match the game-required live library', () => {
  const { genesis, first, inputCommitments } = fixture();
  assert.deepEqual(assertZiffleEpochInputs(first, { input_commitments: [...inputCommitments].reverse() }, [genesis]), inputCommitments);
  const substituted = [...inputCommitments]; substituted[3] = 'ziffle:genesis-a:6';
  const forged = { ...first, inputDeck: buildZiffleInputDeck([genesis], substituted) };
  assert.doesNotThrow(() => assertZiffleEpochHistory(forged, [genesis]), 'membership alone permits another unspent subset');
  assert.throws(() => assertZiffleEpochInputs(forged, { inputCommitments }, [genesis]), /locally required library/);
  assert.throws(() => assertZiffleEpochInputs(first, { inputCommitments: inputCommitments.slice(1) }, [genesis]), /locally required library/);
});

test('another owner or unknown epoch cannot supply the live library inputs', () => {
  const { genesis, other } = fixture();
  for (const source of [`ziffle:${other.deckHash}:0`, 'ziffle:unknown:0', 'ziffle:genesis-a:8',
    'ziffle:genesis-a:-1', 'ziffle:genesis-a:1.5', 'ziffle:genesis-a:9007199254740992']) {
    assert.throws(() => buildZiffleInputDeck([genesis], [source]), /not in an accepted encrypted deck/);
  }
});

test('cryptographic verification must resolve to the signed owner genesis', () => {
  const { genesis, first } = fixture();
  const verified = { deckCount: first.deckCount, deckHash: first.deckHash, rootDeckHash: genesis.deckHash,
    rootContext: genesis.context, universeCount: genesis.deckCount };
  assert.doesNotThrow(() => assertZiffleEpochVerification(first, verified, [genesis]));
  for (const change of [{ deckCount: 3 }, { deckHash: 'other-output' }, { rootDeckHash: 'other-owner' },
    { rootContext: 'other-match' }, { universeCount: 9 }]) {
    assert.throws(() => assertZiffleEpochVerification(first, { ...verified, ...change }, [genesis]), /signed initial deck/);
  }
});

test('accepted history includes this owner only and extends exact previous epochs', () => {
  const { genesis, other, first, second } = fixture();
  const match = { ziffleCeremonies: [genesis, other] };
  const unrelated = { ...first, owner: 1 };
  const accepted = acceptedZiffleEpochs(match, [{ audit: { shuffleProofs: [unrelated, first] } }], 0, [second]);
  assert.deepEqual(accepted, [genesis, first, second]);
  assert.throws(() => acceptedZiffleEpochs(match, [{ audit: { shuffleProofs: [second] } }], 0), /accepted signed transcript/);
  assert.throws(() => acceptedZiffleEpochs(match, [{ audit: { shuffleProofs: [{ ...first, inputDeck: null }] } }], 0), /legacy shuffles/);
});

test('opaque engine material names the epoch and exact input set, with no permutation', () => {
  const { first, inputCommitments } = fixture();
  const material = ziffleEpochMaterial(first, { random_count_before: 21 }, inputCommitments);
  assert.deepEqual(material, { owner: 0, deckHash: 'first-a', count: 4, randomCountBefore: 21, expectedInputs: inputCommitments });
  assertNoIdentityMapping(material);
  for (const value of [undefined, -1, 1.5, 'not-a-number']) {
    assert.throws(() => ziffleEpochMaterial(first, { randomCountBefore: value }, inputCommitments), /random counter/);
  }
});

test('a repeated context may deduplicate identical proof contents but cannot hide a fork', () => {
  const { genesis, first } = fixture();
  const match = { ziffleCeremonies: [genesis] };
  const actions = [{ audit: { shuffleProofs: [first] } }];
  assert.deepEqual(acceptedZiffleEpochs(match, actions, 0, [structuredClone(first)]), [genesis, first]);
  for (const change of [
    proof => { proof.deckHash = 'different-output'; },
    proof => { proof.steps[0].proofHex = 'different-proof'; },
    proof => { proof.inputDeck.sources[0].position = 1; },
  ]) {
    const fork = structuredClone(first); change(fork);
    assert.throws(() => acceptedZiffleEpochs(match, actions, 0, [fork]), /context|fork|history|differs/);
  }
});
