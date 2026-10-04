import test from 'node:test';
import assert from 'node:assert/strict';
import { publicationIdentity, attributePublication } from './catalog-publication-timing.mjs';

test('publication attribution requires both exact rendered states after verified acceptance', () => {
  const action = { submittedAt: 100, publicationTargets: [
    { sequence: 4, identity: 'a' }, { sequence: 4, identity: 'b' },
  ] };
  const event = (at, sequence, identity, pendingVerification = false) => ({ at, sequence, identity, pendingVerification });
  const events = [
    [event(99, 4, 'a'), event(110, 3, 'a'), event(120, 4, 'a', true), event(130, 4, 'old'), event(150, 4, 'a')],
    [event(140, 4, 'b', true), event(180, 4, 'b'), event(200, 4, 'b')],
  ];
  assert.equal(attributePublication(action, events).callerThroughRenderEffectMs, 80);
  assert.equal(attributePublication(action, [events[0], []]), null);
  assert.equal(attributePublication({ submittedAt: 100 }, events), null);
});

test('raw and E2E projections identify the same state without ignoring card or decision changes', () => {
  const raw = { snapshot_id: 20, __priority_revision: 6, turn_number: 2, decision: { kind: 'priority', player: 1, actions: [{ index: 0 }] },
    players: [{ id: 0, life: 20, hand_cards: [{ id: 7, name: 'Card' }], battlefield: [{ id: 8, name: 'Land', tapped: false }] }] };
  const observed = { ...raw, priority_revision: 6, decision: { kind: 'priority', player: 1 }, decisionDetail: raw.decision,
    players: [{ ...raw.players[0], hand_size: 1, library_size: null, graveyard_size: 0 }] };
  // The E2E projection maps an absent library size to null.
  assert.equal(publicationIdentity(raw), publicationIdentity(observed));
  assert.notEqual(publicationIdentity(raw), publicationIdentity({ ...raw, snapshot_id: 19 }));
  assert.notEqual(publicationIdentity(raw), publicationIdentity({ ...raw, decision: { ...raw.decision, player: 0 } }));
  observed.players[0].battlefield = [{ id: 8, name: 'Land', tapped: true }];
  assert.notEqual(publicationIdentity(raw), publicationIdentity(observed));
});
