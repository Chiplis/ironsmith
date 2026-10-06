import test from 'node:test';
import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';
import { verifyEndOfMatchDisclosuresWithGame } from '../src/lib/audit-replay.js';
import { CURRENT_AUDIT_PROTOCOL_VERSION, CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION,
  buildPrivateDeckManifest, buildDeckSlotOpening, publicDeckManifest,
  createAuditSessionKey, exportAuditPublicKey, signAuditPayload } from '../src/lib/multiplayer-audit.js';

async function fixture() {
  const key = await createAuditSessionKey(webcrypto);
  const manifest = await buildPrivateDeckManifest({ matchId: 'disclosure-replay', owner: 1,
    deck: Array(5).fill('Barbarian Ring') }, webcrypto);
  const opening = { ...await buildDeckSlotOpening({ manifest, slot: 4 }, webcrypto),
    position: 51, positionCommitment: 'ziffle:current:51',
    originPosition: 23, originPositionCommitment: 'ziffle:initial:23' };
  const requirement = { type: 'public_open', owner: 1, slot: 4, commitment: opening.commitment,
    publicSlot: 51, publicCommitment: opening.positionCommitment,
    originSlot: 23, originCommitment: opening.originPositionCommitment };
  let engineChecks = 0;
  const game = {
    exportPublicAuditCheckpoint: async () => ({ version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION }),
    uiState: async () => ({ game_over: true, players: [{ id: 0 }, { id: 1, has_lost: true }] }),
    endOfMatchDisclosureRequirements: async owner => owner === 1 ? [requirement] : [],
    verifyEndOfMatchDisclosure: async () => { engineChecks++; return { violations: [], missing: [] }; },
  };
  const transcript = { protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
    matchId: 'disclosure-replay', match: { protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION, players: [{ name: 'Alice' }, {
    name: 'Bob', index: 1, auditPublicKey: await exportAuditPublicKey(key, webcrypto),
    deckAuditManifest: publicDeckManifest(manifest),
  }] }, endOfMatchDisclosures: [] };
  async function check(candidate) {
    const payload = { domain: 'ironsmith-end-of-match-disclosure-v1', matchId: transcript.matchId,
      player: 1, openings: [candidate] };
    transcript.endOfMatchDisclosures = [{ player: 1, disclosure: { ...payload,
      signature: await signAuditPayload(key, payload, webcrypto) } }];
    return verifyEndOfMatchDisclosuresWithGame({ game, transcript, cryptoImpl: webcrypto });
  }
  return { game, transcript, opening, check, engineChecks: () => engineChecks };
}

test('replay binds signed final openings to retained engine obligations before checking claims', async () => {
  const h = await fixture();
  const reports = await h.check(h.opening);
  assert.equal(reports[0].replayVerdict.status, 'verified');
  assert.equal(h.engineChecks(), 1);
});

test('a valid signature and salted slot cannot substitute or omit the required original identity', async () => {
  const h = await fixture();
  const withoutOrigin = { ...h.opening };
  delete withoutOrigin.originPosition; delete withoutOrigin.originPositionCommitment;
  const withoutPosition = { ...withoutOrigin };
  delete withoutPosition.position; delete withoutPosition.positionCommitment;
  for (const opening of [
    { ...h.opening, originPosition: 24, originPositionCommitment: 'ziffle:initial:24' },
    { ...h.opening, owner: 0 }, withoutOrigin, withoutPosition,
  ]) {
    const reports = await h.check(opening);
    assert.notEqual(reports[0].replayVerdict.status, 'verified');
  }
  assert.equal(h.engineChecks(), 0, 'invalid identity never reaches claim verification');
});

test('replay detects an omitted required disclosure independently of transcript entries', async () => {
  const h = await fixture();
  const reports = await verifyEndOfMatchDisclosuresWithGame({ game: h.game, transcript: h.transcript, cryptoImpl: webcrypto });
  assert.equal(reports.length, 1);
  assert.equal(reports[0].player, 1);
  assert.equal(reports[0].replayVerdict.status, 'missing');
});
