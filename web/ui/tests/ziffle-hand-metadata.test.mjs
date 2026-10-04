import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const source = readFileSync(new URL('../src/hooks/peer-lobby/validation.js', import.meta.url), 'utf8');
const shared = readFileSync(new URL('../src/hooks/peer-lobby/shared.js', import.meta.url), 'utf8');
const slice = (text, start, end) => {
  const a = text.indexOf(start), b = text.indexOf(end, a + start.length);
  assert.ok(a >= 0 && b > a);
  return text.slice(a, b);
};
const helpers = ['ziffleDeckHashFromCommitment', 'zifflePositionFromCommitment', 'ziffleIdentityPositionFromSources']
  .map(name => slice(shared, `export function ${name}(`, '\nexport ').replace('export ', '')).join('\n');
const body = helpers
  + slice(source, '  function handIdsForRevealKey(', '  function isInspectorOnlyViewedCards(')
  + slice(source, '  async function revealLocalZiffleHandInner(', '  async function buildZiffleCeremoniesForPayload(')
  + '\nreturn revealLocalZiffleHandInner;';

test('hand hydration completes even when executable checkpoint export is unsupported', async () => {
  const calls = [];
  const opening = { owner: 0, slot: 2, card: 'Mountain', commitment: 'salted-card' };
  const state = { players: [{ id: 0, hand: [101] }],
    objects: [{ id: 101, name: 'Hidden Card', zone: 'hand', hiddenCard: opening }] };
  const game = {
    exportSyncCheckpoint: async () => { throw new Error('registered continuous effect requires an approved executable identity graph'); },
    getHiddenCardState: async () => { calls.push('metadata'); return state; },
    exportHiddenCardOpening: async () => opening,
    ziffleRevealCard() {}, ziffleRevealCards() {}, revealHiddenPosition() {},
    revealHiddenObject: async value => calls.push(value), setPerspective: async () => {},
  };
  const context = {
    gameRef: { current: game }, liveZiffleCeremoniesRef: { current: new Map() },
    multiplayerRef: { current: {} }, resolveLocalPlayerIndex: () => 0,
    privateDeckManifestForOwner: () => ({ owner: 0 }), currentAuditMatchId: () => 'match',
    ziffleHandRevealKeyRef: { current: '' }, ziffleHandRevealQuickKeyRef: { current: '' },
    wasmObjectIdArg: value => value, zifflePositionForObjectId: () => null,
    zifflePositionForOriginalSlot: () => null,
    localRevealedOpeningForExport: () => opening, openingHasZifflePosition: () => false,
    cloneMultiplayerPayload: structuredClone, rememberLocalRevealedOpening: () => {},
    rememberZiffleOpeningPosition: () => {},
  };
  const hydrate = new Function(...Object.keys(context), body)(...Object.values(context));
  await hydrate({ auditMatchId: 'match', ziffleCeremonies: [{}] }, { updateState: false });
  assert.equal(calls[0], 'metadata');
  assert.equal(calls[1].objectId, 101);
  assert.equal(calls[1].cardName, 'Mountain');
  assert.equal(calls[1].commitment, 'salted-card');
});
