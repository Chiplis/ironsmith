import test from 'node:test';
import assert from 'node:assert/strict';
import { authorizationHarness, ceremony, commitment, opening, shuffleProof } from './ziffle-reveal-authorization-harness.mjs';

test('signed attached openings cannot manufacture authorization or bypass the live decision', async () => {
  const h = authorizationHarness();
  h.message.actionAuthorization.requirements = [opening(23)];
  assert.equal(await h.authorize(h.message, 0, 0, [23], ceremony), false);
  h.message.actionAuthorization.command = { type: 'not_a_decision' };
  assert.equal(await h.authorize(h.message, 0, 0, [23], ceremony), false);
});

test('historical single-card openings cannot reopen arbitrary positions or later ceremonies', async () => {
  const h = authorizationHarness({ stored: [opening(2, { type: 'public_open' })], lastSequence: 8 });
  for (const position of [0, 1, 3, 59]) assert.equal(await h.authorize(h.message, 0, 0, [position], ceremony), false);
  assert.equal(await h.authorize(h.message, 0, 0, [2], ceremony), true);
  assert.equal(await h.authorize(h.message, 0, 0, [2], { ...ceremony, deckHash: 'later' }), false);
  const window = authorizationHarness({ stored: [{ type: 'private_view_window', owner: 0, viewer: 0, zone: 'library', count: 60 }], lastSequence: 8 });
  assert.equal(await window.authorize(window.message, 0, 0, [23], ceremony), false);
});

test('exact private/public openings and library searches preserve owner/viewer and ceremony boundaries', async () => {
  const requirements = Array.from({ length: 60 }, (_, position) => opening(position));
  const h = authorizationHarness({ requirements });
  assert.equal(await h.authorize(h.message, 0, 0, [0, 2, 59], ceremony), true);
  assert.equal(h.direct(requirements, 1, 0, [2], ceremony), false);
  assert.equal(h.direct([opening(2, { type: 'public_open' })], 1, 0, [2], ceremony), true);
  assert.equal(h.direct(requirements, 0, 0, [60], ceremony), false);
  assert.equal(h.direct(requirements, 0, 0, [2], { ...ceremony, deckHash: 'later' }), false);
});

test('metadata binds the exact committed object despite different peer runtime IDs', async () => {
  const checkpoint = { objects: [2, 9].map(position => ({ id: 200 + position, name: 'Hidden Card', zone: 'hand',
    hiddenCard: { owner: 0, slot: position, commitment: `original-${position}`, publicSlot: position, publicCommitment: commitment(position) } })) };
  const requirement = opening(2, { object_id: 999, slot: 2, commitment: 'original-2', public_commitment: commitment(2) });
  const h = authorizationHarness({ checkpoint });
  assert.equal(await h.metadata([requirement], 0, 0, [2], ceremony), true);
  assert.equal(await h.metadata([requirement], 0, 0, [9], { ...ceremony, afterOrder: Array.from({ length: 60 }, (_, position) => 200 + position) }), false);
  const raw = { ...requirement }; delete raw.public_commitment;
  assert.equal(await h.metadata([raw], 0, 0, [2], ceremony), false, 'historical unpinned identity cannot authorize a later position');
  assert.equal(await h.metadata([raw], 0, 0, [2], ceremony, true), true, 'a trusted live preview can resolve its local committed object');
});

test('mulligan openings require trusted exact shuffle orders, owner, and action context', async () => {
  const before = Array.from({ length: 60 }, (_, position) => 100 + position), after = before.slice().reverse();
  const requirement = { id: 'mulligan', type: 'verifiable_shuffle', owner: 0, zone: 'library', count: 53, before_order: before, after_order: after };
  const shuffled = { ...ceremony, context: 'match:action:8:shuffle:mulligan:0:library', beforeOrder: before, afterOrder: after };
  const h = authorizationHarness({ requirements: [requirement] });
  h.message.actionAuthorization.shuffleProofs = [shuffleProof(requirement)];
  assert.equal(await h.authorize(h.message, 0, 0, [53, 59], shuffled), true);
  assert.equal(await h.authorize(h.message, 0, 0, [52], shuffled), false);
  assert.equal(h.direct([requirement], 1, 0, [53], shuffled, 8), false);
  assert.equal(h.direct([requirement], 0, 0, [53], { ...shuffled, context: 'match:action:9:shuffle:mulligan:0:library' }, 8), false);
  const missing = { ...requirement }; delete missing.after_order;
  const forged = authorizationHarness({ requirements: [missing] });
  forged.message.actionAuthorization.requirements = [requirement];
  assert.equal(await forged.authorize(forged.message, 0, 0, [53], shuffled), false, 'attached orders cannot enrich incomplete trusted requirements');
});

test('shuffle-dependent openings use the verified seeded preview and never the preliminary cards', async () => {
  const before = Array.from({ length: 60 }, (_, i) => i + 100);
  const requirement = { id: 'mulligan', type: 'verifiable_shuffle', owner: 0, zone: 'library', count: 53,
    beforeOrder: before, afterOrder: before.slice().reverse() };
  const h = authorizationHarness({ requirements: [requirement, opening(2)], materialRequirements: [requirement, opening(9)] });
  const proof = shuffleProof(requirement);
  // The peer's object order must not enter the trusted engine preview.
  proof.beforeOrder = [999]; proof.afterOrder = [888];
  h.message.actionAuthorization.shuffleProofs = [proof];
  assert.equal(await h.authorize(h.message, 0, 0, [9], ceremony), true);
  assert.equal(await h.authorize(h.message, 0, 0, [2], ceremony), false);
  assert.deepEqual(h.materialCalls[0].material, { seeds: ['shuffled'], libraryEpochs: [], libraryEpochOpenings: [], libraryShuffles: [{ owner: 0,
    beforeOrder: before, afterOrder: requirement.afterOrder }] });
  for (const mutate of [
    p => { p.context = p.context.replace('action:8', 'action:9'); },
    p => { p.keyContext = 'different-match'; },
    p => { p.keys = ['forged-roster']; },
    p => { p.deckCount = 59; },
    p => { p.requirementId = 'other'; },
    p => { p.steps[0].proofHex = 'invalid'; },
    p => { p.deckHash = 'not-verified'; },
  ]) {
    const changed = structuredClone(proof); mutate(changed);
    h.message.actionAuthorization.shuffleProofs = [changed];
    assert.equal(await h.authorize(h.message, 0, 0, [9], ceremony), false);
  }
  delete h.message.actionAuthorization.shuffleProofs;
  assert.equal(await h.authorize(h.message, 0, 0, [2], ceremony), false, 'a seedless request cannot open preliminary draw cards');
});

test('owner hand hydration survives a concealed cast that the remote engine cannot preview', async () => {
  const h = authorizationHarness({ previewError: true, visible: [2] });
  assert.equal(await h.authorize(h.message, 0, 0, [2], ceremony), true);
  assert.equal(await h.authorize(h.message, 0, 0, [3], ceremony), false);
  h.message.actionAuthorization.requesterIndex = 1;
  assert.equal(await h.authorize(h.message, 1, 0, [2], ceremony), false);
});

test('one action cannot reveal or commit multiple valid counterfactual shuffles, including concurrent requests', async () => {
  const beforeOrder = Array.from({ length: 60 }, (_, i) => i + 100);
  const requirement = { id: 'mulligan', type: 'verifiable_shuffle', owner: 0, zone: 'library', count: 53,
    beforeOrder, afterOrder: beforeOrder.slice().reverse() };
  const first = shuffleProof(requirement), alternative = shuffleProof(requirement);
  alternative.deckHash = alternative.steps[0].deckHex = 'another-valid-deck';
  const h = authorizationHarness({ requirements: [requirement], materialRequirements: [opening(9)] });
  const messages = [first, alternative].map(proof => {
    const message = structuredClone(h.message); message.actionAuthorization.shuffleProofs = [proof]; return message;
  });
  const results = await Promise.all(messages.map(message => h.authorize(message, 0, 0, [9], ceremony)));
  assert.equal(results.filter(Boolean).length, 1);
  const accepted = results[0] ? first : alternative, rejected = results[0] ? alternative : first;
  await h.verify([requirement], [accepted], { seq: 8 });
  await assert.rejects(h.verify([requirement], [rejected], { seq: 8 }), /conflicts with previously authorized/);
  await assert.rejects(h.verify([requirement], [accepted], { seq: 9 }), /not bound to this action sequence/);
});

test('outbound requests retain exact position, requester and ceremony authorization', async () => {
  const pending = new Map([['material', { peerId: 'peer-0', targetSeat: 0, seq: 8,
    actionIntent: { signature: 'valid' }, requirements: [opening(2)] }]]);
  const h = authorizationHarness({ pending });
  const message = { cryptoMaterialRequestId: 'material', requesterPeerId: 'peer-0' };
  assert.equal(await h.outbound(message, 0, 0, [2], ceremony), true);
  assert.equal(await h.outbound(message, 0, 0, [3], ceremony), false);
  assert.equal(await h.outbound(message, 0, 0, [2], { ...ceremony, deckHash: 'later' }), false);
  assert.equal(await h.outbound(message, 1, 0, [2], ceremony), false);
});

test('trusted current openings authorize only their exact immutable genesis anchors', async () => {
  const current = opening(51, { commitment: commitment(51, 'fetch'), originSlot: 23, originCommitment: commitment(23) });
  const h = authorizationHarness({ requirements: [current] });
  assert.equal(await h.authorize(h.message, 0, 0, [23], ceremony), true);
  for (const position of [2, 22, 24, 51]) assert.equal(await h.authorize(h.message, 0, 0, [position], ceremony), false);
  assert.equal(h.direct([current], 1, 0, [23], ceremony), false);
  assert.equal(h.direct([{ ...current, type: 'public_open' }], 1, 0, [23], ceremony), true);
  assert.equal(h.direct([current], 0, 0, [23], { ...ceremony, deckHash: 'other-genesis' }), false);
  const forged = authorizationHarness();
  forged.message.actionAuthorization.requirements = [current];
  assert.equal(await forged.authorize(forged.message, 0, 0, [23], ceremony), false);
});

test('authorized metadata remaps runtime IDs but cannot grant an unrelated card origin', async () => {
  const checkpoint = { objects: [23, 24].map((origin, index) => ({ id: 211 + index, zone: 'hand', name: 'Barbarian Ring',
    hiddenCard: { owner: 0, slot: 4 - index, commitment: `original-${4 - index}`, publicSlot: 51 - index,
      publicCommitment: commitment(51 - index, 'fetch'), originSlot: origin, originCommitment: commitment(origin) } })) };
  const requirement = opening(51, { type: 'public_open', object_id: 999, slot: 4, commitment: 'original-4',
    public_commitment: commitment(51, 'fetch') });
  const h = authorizationHarness({ checkpoint, requirements: [requirement] });
  assert.equal(await h.metadata([requirement], 0, 0, [23], ceremony), true);
  assert.equal(await h.metadata([requirement], 0, 0, [24], ceremony), false);
  const forged = { ...requirement, originSlot: 24, originCommitment: commitment(24) };
  assert.equal(await h.metadata([forged], 0, 0, [24], ceremony), false, 'metadata never trusts an attached anchor');
  assert.equal(await h.metadata([requirement], 0, 0, [23], { ...ceremony, deckHash: 'other' }), false);
});

test('owner visible-state authorization exposes hand origins and excludes unrelated library anchors', async () => {
  const checkpoint = { players: [{ id: 0, hand: [211] }], objects: [23, 24].map((origin, index) => ({
    id: 211 + index, zone: index ? 'library' : 'hand', name: 'Hidden Card', hiddenCard: { owner: 0, slot: 4 - index,
      commitment: `original-${4 - index}`, publicSlot: 51 - index, publicCommitment: commitment(51 - index, 'fetch'),
      originSlot: origin, originCommitment: commitment(origin) } })) };
  const h = authorizationHarness({ checkpoint });
  assert.deepEqual([...(await h.visiblePositions(0, 'deck'))], [23]);
  assert.deepEqual([...(await h.visiblePositions(1, 'deck'))], []);
  assert.deepEqual([...(await h.visiblePositions(0, 'other'))], []);
});

test('visible-state authorization follows face-down exile look permission, not ownership', async () => {
  // Alice (0) owns both cards. Bob (1) exiled 212 face down with Gonti and may
  // look at it; 211 is in Alice's hand.
  const checkpoint = { players: [{ id: 0, hand: [211] }], exile: [212], objects: [
    { id: 211, zone: 'hand', name: 'Hidden Card', hiddenCard: { owner: 0, slot: 4, commitment: 'original-4',
      originSlot: 23, originCommitment: commitment(23) } },
    { id: 212, zone: 'exile', name: 'Hidden Card', hiddenCard: { owner: 0, slot: 5, commitment: 'original-5',
      originSlot: 31, originCommitment: commitment(31) } },
  ] };
  const h = authorizationHarness({ checkpoint, viewable: (id, viewer) => id === 212 ? viewer === 1 : viewer === 0 });
  assert.deepEqual([...(await h.visiblePositions(0, 'deck', 1))], [31], 'the entitled viewer may reopen the exiled card');
  assert.deepEqual([...(await h.visiblePositions(0, 'deck'))], [23], 'the owner may not reopen a card it may not look at');
  const stranger = authorizationHarness({ checkpoint, viewable: (id, viewer) => id === 211 && viewer === 0 });
  assert.deepEqual([...(await stranger.visiblePositions(0, 'deck', 1))], [], 'no look permission, no tokens');
});

test('a non-owner requester reaches the visible-state fallback only with its own view rights', async () => {
  const h = authorizationHarness({ previewError: true, visible: [2], visibleToOthers: [7] });
  h.message.actionAuthorization.requesterIndex = 1;
  assert.equal(await h.authorize(h.message, 1, 0, [7], ceremony), true);
  assert.equal(await h.authorize(h.message, 1, 0, [2], ceremony), false);
});

test('search and mulligan previews authorize exact genesis anchors, never window counts', async () => {
  const requirements = [8, 19, 23].map((origin, index) => opening(index, { commitment: commitment(index, 'fetch'),
    originSlot: origin, originCommitment: commitment(origin) }));
  const h = authorizationHarness({ requirements });
  assert.equal(await h.authorize(h.message, 0, 0, [8, 19, 23], ceremony), true);
  assert.equal(await h.authorize(h.message, 0, 0, [8, 20, 23], ceremony), false);
  assert.equal(h.direct([{ type: 'private_view_window', owner: 0, viewer: 0, count: 60,
    originSlot: 24, originCommitment: commitment(24) }], 0, 0, [24], ceremony), false);
});
