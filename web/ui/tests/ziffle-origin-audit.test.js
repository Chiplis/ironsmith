import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { webcrypto } from 'node:crypto';
import { buildDeckSlotOpening, buildPrivateDeckManifest, buildZiffleOpeningProof,
  canonicalJson, verifyCardOpeningAgainstManifest, assertZiffleOpeningOriginMatchesMetadata,
  ZIFFLE_OPENING_PROOF_TYPE } from '../src/lib/multiplayer-audit.js';

const source = readFileSync(new URL('../src/lib/multiplayer-audit.js', import.meta.url), 'utf8');
function between(start, end) {
  const a = source.indexOf(start), b = source.indexOf(end, a + start.length);
  assert.ok(a >= 0 && b > a, start);
  return source.slice(a, b).replaceAll('export function ', 'function ');
}
// Real salted-manifest and opening verification; the cryptographic backend
// callback returns a fixed already-verified genesis slot.
const verify = new Function('canonicalJson', 'verifyCardOpeningAgainstManifest', 'ZIFFLE_OPENING_PROOF_TYPE',
  `${between('function ziffleDeckHashFromCommitment(', 'export function buildZiffleOpeningProof(')}
  ${between('async function verifyAuditOpenings(', 'function privateViewDisclosurePayload(')}
  return verifyAuditOpenings;`)(canonicalJson, verifyCardOpeningAgainstManifest, ZIFFLE_OPENING_PROOF_TYPE);

async function fixture() {
  const deck = Array(61).fill('Mountain');
  for (const slot of [2, 3, 4]) deck[slot] = 'Barbarian Ring';
  const manifest = await buildPrivateDeckManifest({ matchId: 'origin-match', owner: 1, deck }, webcrypto);
  const keys = [0, 1].map(player => ({ player, publicKeyHex: `key-${player}`, ownershipProofHex: `ownership-${player}` }));
  const genesis = { owner: 1, deckCount: 61, context: 'origin-match', keyContext: 'origin-match', deckHash: 'initial', keys, steps: [] };
  const current = { ...genesis, deckCount: 53, context: 'origin-match:action:42:shuffle:fetch:1:library', deckHash: 'fetch',
    beforeOrder: Array.from({ length: 53 }, (_, i) => 100 + i), afterOrder: Array.from({ length: 53 }, (_, i) => 152 - i), authenticatedOrder: true };
  const opening = { ...await buildDeckSlotOpening({ manifest, slot: 4 }, webcrypto), objectId: 212,
    position: 51, positionCommitment: 'ziffle:fetch:51', ziffleContext: current.context,
    originPosition: 23, originPositionCommitment: 'ziffle:initial:23' };
  const tokens = keys.map(key => ({ ...key, tokenHex: 'token', proofHex: 'proof' }));
  opening.ziffleReveal = buildZiffleOpeningProof({ opening, ceremony: genesis, tokens, compact: true });
  const calls = [];
  const check = candidate => verify({ openings: [candidate], manifests: new Map([[1, manifest]]),
    players: new Map([[0, {}], [1, {}]]), ziffleCeremonies: [genesis, current], expectedZiffleKeys: keys,
    expectedMatchId: 'origin-match', seq: 166, verifyZiffleOpening: async input => { calls.push(input); return { originalSlot: 4 }; } }, webcrypto);
  return { opening, manifest, genesis, current, tokens, calls, check };
}

test('standalone genesis proof binds later 53-card shuffle to original 61-card manifest', async () => {
  const h = await fixture(); await h.check(h.opening);
  assert.equal(h.calls.length, 1);
  assert.equal(h.calls[0].proof.position, 23);
  assert.equal(h.calls[0].ceremony.deckCount, 61);
  assert.equal(h.calls[0].proof.originalSlot, 4);
  assert.equal(h.opening.position, 51);
});

test('standalone verifier rejects another committed copy of the same card', async () => {
  const h = await fixture();
  for (const slot of [2, 3]) {
    const forged = { ...h.opening, ...await buildDeckSlotOpening({ manifest: h.manifest, slot }, webcrypto) };
    forged.ziffleReveal = buildZiffleOpeningProof({ opening: forged, ceremony: h.genesis, shuffleOriginalSlot: 4, tokens: h.tokens });
    await assert.rejects(h.check(forged), /different committed slot/);
  }
});

test('standalone verifier rejects public object IDs as a substitute for genesis proof', async () => {
  const h = await fixture();
  const forged = { ...h.opening, objectId: h.current.afterOrder[51], shuffleObjectId: h.current.beforeOrder[2] };
  delete forged.originPosition; delete forged.originPositionCommitment;
  for (const proof of [null, buildZiffleOpeningProof({ opening: forged, ceremony: h.current, shuffleOriginalSlot: 2, tokens: h.tokens })]) {
    if (proof) forged.ziffleReveal = proof; else delete forged.ziffleReveal;
    await assert.rejects(h.check(forged), /immutable genesis origin/);
  }
  assert.equal(h.calls.length, 0);
});

test('standalone origin proof validates owner, position, ceremony, keys and absence of orders', async () => {
  const h = await fixture();
  for (const mutate of [
    o => { o.originPosition = 24; }, o => { delete o.originPosition; },
    o => { o.originPositionCommitment = 'ziffle:other:23'; },
    o => { o.ziffleReveal.position = 24; }, o => { o.ziffleReveal.owner = 0; },
    o => { o.ziffleReveal.context = h.current.context; }, o => { o.ziffleReveal.keyContext = 'other'; },
    o => { o.ziffleReveal.keys = [{ player: 0, publicKeyHex: 'forged' }]; },
    o => { o.ziffleReveal.beforeOrder = h.current.beforeOrder; o.ziffleReveal.afterOrder = h.current.afterOrder; },
  ]) {
    const forged = structuredClone(h.opening); mutate(forged); await assert.rejects(h.check(forged));
  }
  assert.equal(h.calls.length, 0);
});

test('current metadata must independently match origin even across runtime ID changes', async () => {
  const h = await fixture();
  const metadata = { objectId: 212, owner: 1, slot: 4, commitment: h.opening.commitment,
    publicSlot: 51, publicCommitment: 'ziffle:fetch:51', originSlot: 23, originCommitment: 'ziffle:initial:23' };
  assert.deepEqual(assertZiffleOpeningOriginMatchesMetadata(h.opening, metadata),
    { originPosition: 23, originPositionCommitment: 'ziffle:initial:23' });
  for (const changed of [null, { ...metadata, owner: 0 }, { ...metadata, originSlot: 22, originCommitment: 'ziffle:initial:22' },
    { ...metadata, originCommitment: 'ziffle:earlier:23' }, { ...metadata, originCommitment: '' }]) {
    assert.throws(() => assertZiffleOpeningOriginMatchesMetadata(h.opening, changed), /origin/);
  }
});
