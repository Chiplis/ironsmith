import test from 'node:test';
import assert from 'node:assert/strict';
import { authorizationHarness } from './ziffle-reveal-authorization-harness.mjs';
import { buildZiffleInputDeck } from '../src/lib/ziffle-private-epochs.js';

function fixture() {
  const genesis = { owner: 0, deckCount: 8, context: 'match', deckHash: 'root',
    steps: [{ shuffler: 0, deckHex: 'root', proofHex: 'valid' }] };
  const requirement = { id: 'fetch', type: 'verifiable_shuffle', owner: 0, zone: 'library',
    randomCountBefore: 12, beforeOrder: [1, 2, 3, 4], afterOrder: [4, 3, 2, 1],
    inputCommitments: [0, 2, 4, 7].map(position => `ziffle:root:${position}`) };
  const proof = { owner: 0, zone: 'library', requirementId: 'fetch', deckCount: 4,
    context: 'match:action:8:shuffle:fetch:0:library', keyContext: 'match', deckHash: 'new',
    keys: ['signed-roster'], steps: [{ shuffler: 0, deckHex: 'new', proofHex: 'valid' }],
    inputDeck: buildZiffleInputDeck([genesis], requirement.inputCommitments) };
  const opening = { type: 'private_open', owner: 0, viewer: 0, zone: 'hand', slot: 3,
    commitment: 'ziffle:new:3', originSlot: 3, originCommitment: 'ziffle:new:3' };
  const h = authorizationHarness({ match: { protocolVersion: 15, ziffleCeremonies: [genesis] },
    requirements: [requirement], materialRequirements: [requirement, opening] });
  h.message.actionAuthorization.shuffleProofs = [proof];
  return { h, proof, requirement, opening };
}

test('new shuffle authorization previews opaque epochs and permits only actual output openings', async () => {
  const { h, proof } = fixture();
  assert.equal(await h.authorize(h.message, 0, 0, [3], proof), true);
  assert.equal(await h.authorize(h.message, 0, 0, [2], proof), false);
  assert.equal(await h.authorize(h.message, 1, 0, [3], proof), false);
  const material = h.materialCalls[0].material;
  assert.deepEqual(material.seeds, []);
  assert.deepEqual(material.libraryShuffles, []);
  assert.deepEqual(material.libraryEpochs, [{ owner: 0, deckHash: 'new', count: 4, randomCountBefore: 12,
    expectedInputs: ['ziffle:root:0', 'ziffle:root:2', 'ziffle:root:4', 'ziffle:root:7'] }]);
  assert.doesNotMatch(JSON.stringify(material), /beforeOrder|afterOrder|objectId/);
});

test('live private shuffle binding rejects substitutions, changed history and linked legacy proofs', async () => {
  for (const change of [
    proof => { proof.inputDeck.sources[0].position = 1; },
    proof => { proof.inputDeck.epochs[0].steps[0].deckHex = 'other-root'; },
    proof => { proof.beforeOrder = [1, 2, 3, 4]; },
    proof => { delete proof.inputDeck; },
  ]) {
    const { h, proof, requirement } = fixture(); change(proof);
    await assert.rejects(h.verify([requirement], [proof], { seq: 8 }), /inputs|history|mapping|private shuffle/);
    assert.equal(await h.authorize(h.message, 0, 0, [3], proof), false);
  }
});
