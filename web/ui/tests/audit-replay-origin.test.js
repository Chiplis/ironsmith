import test from 'node:test';
import assert from 'node:assert/strict';
import { applyAuditReplayActionWithGame, startAuditTranscriptReplayWithGame } from '../src/lib/audit-replay.js';
import { CURRENT_AUDIT_PROTOCOL_VERSION, CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION } from '../src/lib/multiplayer-audit.js';

function replayGame() {
  const calls = [];
  const game = {
    startMatch: async () => {},
    getHiddenCardState: async () => ({ objects: [{ id: 212, stableId: 85, hiddenCard: {
      owner: 1, slot: 4, commitment: 'salted-ring-4', publicSlot: 51, publicCommitment: 'ziffle:current:51',
      originSlot: 23, originCommitment: 'ziffle:initial:23',
    } }] }),
    revealHiddenPosition: async opening => calls.push(['reveal', opening]),
    previewCryptoRequirements: async () => [],
    dispatch: async command => calls.push(['dispatch', command]),
    exportPublicAuditCheckpoint: async () => ({ version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION }),
  };
  return { game, calls };
}

const opening = { owner: 1, objectId: 211, slot: 4, commitment: 'salted-ring-4', card: 'Barbarian Ring',
  position: 51, positionCommitment: 'ziffle:current:51', originPosition: 23,
  originPositionCommitment: 'ziffle:initial:23', timing: 'pre' };

async function replay(game, candidate) {
  await startAuditTranscriptReplayWithGame({ game, transcript: {
    protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
    match: { protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION, players: [] },
  } });
  return applyAuditReplayActionWithGame({ game, action: { seq: 1,
    command: { type: 'priority_action', action_ref: { kind: 'pass_priority' } },
    audit: { openings: [candidate] } } });
}

test('engine replay binds the original identity before revealing the current position', async () => {
  const { game, calls } = replayGame();
  await replay(game, opening);
  assert.deepEqual(calls.map(call => call[0]), ['reveal', 'dispatch']);
  assert.equal(calls[0][1].position, 51);
  assert.equal(calls[0][1].originalSlot, 4);
  assert.equal(calls[0][1].objectId, 212, 'use the authenticated current object, not the opening\'s retired id');
});

test('post-action replay reopens a revealed card through its authenticated identity after a zone change', async () => {
  const calls = [];
  let objectId = 168;
  const game = {
    startMatch: async () => {},
    getHiddenCardState: async () => ({ objects: [{ id: objectId, stableId: 116,
      name: 'Goblin Guide', zone: objectId === 168 ? 'stack' : 'battlefield', hiddenCard: {
        owner: 1, slot: 22, commitment: 'salted-guide-22', publicSlot: 54,
        publicCommitment: 'ziffle:initial:54', originSlot: 54,
        originCommitment: 'ziffle:initial:54',
      } }] }),
    revealHiddenPosition: async candidate => {
      // The real engine's positional search only selects hidden placeholders;
      // an already revealed card needs the independently authenticated id.
      if (candidate.objectId !== objectId) {
        throw new Error('hidden ziffle position is not present in this engine');
      }
      calls.push(['reveal', candidate]);
    },
    revealHiddenObject: async () => { throw new Error('retired transcript object must not be used'); },
    revealHiddenSlot: async () => { throw new Error('hidden card commitment does not match reveal'); },
    previewCryptoRequirements: async () => [],
    dispatch: async () => { objectId = 169; calls.push(['dispatch']); },
    exportPublicAuditCheckpoint: async () => ({ version: CURRENT_PUBLIC_AUDIT_CHECKPOINT_VERSION }),
  };
  await replay(game, { owner: 1, objectId: 168, slot: 22, card: 'Goblin Guide',
    commitment: 'salted-guide-22', position: 54, positionCommitment: 'ziffle:initial:54',
    originPosition: 54, originPositionCommitment: 'ziffle:initial:54', timing: 'post' });
  assert.deepEqual(calls.map(call => call[0]), ['dispatch', 'reveal']);
  assert.equal(calls[1][1].objectId, 169);
  assert.equal(calls[1][1].commitment, 'salted-guide-22');
});

test('engine replay rejects duplicate current commitment identities before revealing', async () => {
  const { game, calls } = replayGame();
  const checkpoint = await game.getHiddenCardState();
  checkpoint.objects.push({ ...checkpoint.objects[0], id: 213 });
  game.getHiddenCardState = async () => checkpoint;
  await assert.rejects(replay(game, opening), /does not identify one current committed card/);
  assert.deepEqual(calls, []);
});

test('engine replay rejects wrong or omitted lineage before hydrating any card', async () => {
  const { originPosition: _originPosition, originPositionCommitment: _originPositionCommitment, ...withoutOrigin } = opening;
  const { position: _position, positionCommitment: _positionCommitment, ...withoutPositionOrOrigin } = withoutOrigin;
  const candidates = [
    { ...opening, originPosition: 24, originPositionCommitment: 'ziffle:initial:24' },
    { ...opening, originPositionCommitment: 'ziffle:other:23' },
    { ...opening, position: 50, positionCommitment: 'ziffle:current:50' },
    withoutOrigin,
    withoutPositionOrOrigin,
  ];
  for (const candidate of candidates) {
    const { game, calls } = replayGame();
    await assert.rejects(replay(game, candidate), /identity|committed card|missing its current position/);
    assert.deepEqual(calls, [], 'a rejected opening must not reach hydration or dispatch');
  }
});
