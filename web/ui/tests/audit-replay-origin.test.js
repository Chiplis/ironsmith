import test from 'node:test';
import assert from 'node:assert/strict';
import { applyAuditReplayActionWithGame } from '../src/lib/audit-replay.js';

function replayGame() {
  const calls = [];
  const game = {
    exportSyncCheckpoint: async () => ({ objects: [{ id: 212, stableId: 85, hiddenCard: {
      owner: 1, slot: 4, commitment: 'salted-ring-4', publicSlot: 51, publicCommitment: 'ziffle:current:51',
      originSlot: 23, originCommitment: 'ziffle:initial:23',
    } }] }),
    revealHiddenPosition: async opening => calls.push(['reveal', opening]),
    previewCryptoRequirements: async () => [],
    dispatch: async command => calls.push(['dispatch', command]),
    exportPublicAuditCheckpoint: async () => ({}),
  };
  return { game, calls };
}

const opening = { owner: 1, objectId: 211, slot: 4, commitment: 'salted-ring-4', card: 'Barbarian Ring',
  position: 51, positionCommitment: 'ziffle:current:51', originPosition: 23,
  originPositionCommitment: 'ziffle:initial:23', timing: 'pre' };

async function replay(game, candidate) {
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
