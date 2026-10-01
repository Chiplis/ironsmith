import test from 'node:test';
import assert from 'node:assert/strict';
import { calculateOptimisticAction, missingCalculationMaterial } from '../src/lib/optimistic-game-runtime.js';

function harness(requirements) {
  let state = { step: 0, seeds: [], opened: [] }, saved;
  let dispatches = 0, releases = 0;
  const game = {
    createRuntimeSavepoint: async () => { saved = structuredClone(state); return 1; },
    restoreRuntimeSavepoint: async () => { state = saved; },
    releaseRuntimeSavepoint: async () => { releases++; },
    uiState: async () => structuredClone(state),
    exportPublicAuditCheckpoint: async () => ({ step: state.step }),
    previewCryptoRequirements: async () => requirements,
    injectTranscriptRandomSeeds: async material => { state.seeds.push(...material.seeds); },
    hiddenCardOpenState: async id => ({ open: state.opened.includes(Number(id)) }),
    revealHiddenObject: async opening => { state.opened.push(opening.objectId); },
  };
  const calculate = candidate => calculateOptimisticAction(game, { command: { type: 'priority_action' }, ...candidate }, {
    dispatch: async () => { dispatches++; state.step++; return { ...state, crypto: requirements }; },
    requirementsFromState: st => st.crypto,
    checkpointHash: async checkpoint => String(checkpoint.step),
  });
  return { game, calculate, state: () => state, counts: () => ({ dispatches, releases }) };
}

test('missing random material blocks dispatch and restores the entire transaction', async () => {
  const h = harness([{ type: 'fair_random', id: 'coin' }]);
  assert.equal(await h.calculate({}), null);
  assert.deepEqual(h.state(), { step: 0, seeds: [], opened: [] });
  assert.deepEqual(h.counts(), { dispatches: 0, releases: 1 });
});

test('received random result can be calculated before its proof verification', async () => {
  const h = harness([{ type: 'fair_random', id: 'coin' }]);
  const result = await h.calculate({ calculationAudit: { rngReveals: [{ requirementId: 'coin', combinedSeedHex: 'a'.repeat(64) }] } });
  assert.equal(result.publicCheckpointHash, '1');
  assert.deepEqual(h.state().seeds, ['a'.repeat(64)]);
});

test('unknown identity blocks but an existing opening and view metadata need no new exchange', async () => {
  const h = harness([{ type: 'private_view_window', id: 'view' }, { type: 'private_open', id: 'card', objectId: 7 }]);
  assert.equal(await h.calculate({}), null);
  await h.game.revealHiddenObject({ objectId: 7 });
  assert.ok(await h.calculate({}));
  assert.equal(h.counts().dispatches, 1);
});

test('canonical public disclosure is restricted to identities requested by the engine preview', async () => {
  const h = harness([{ type: 'public_open', id: 'card', owner: 1, objectId: 7 }]);
  assert.ok(await h.calculate({ calculationAudit: { openings: [
    { objectId: 7, owner: 1, card: 'Forest', commitment: 'c' },
    { objectId: 8, owner: 1, card: 'Island', commitment: 'secret' },
  ] } }));
  assert.deepEqual(h.state().opened, [7]);
});

test('a mismatched public hash rolls back injected material as well as gameplay', async () => {
  const h = harness([{ type: 'fair_random', id: 'coin' }]);
  assert.equal(await h.calculate({ publicCheckpointHash: 'invalid', calculationAudit: {
    rngReveals: [{ requirementId: 'coin', combinedSeedHex: 'a'.repeat(64) }],
  } }), null);
  assert.deepEqual(h.state(), { step: 0, seeds: [], opened: [] });
});

test('unsupported material requirements fail closed', async () => {
  const h = harness([]);
  assert.ok(await missingCalculationMaterial(h.game, [{ type: 'unknown_crypto' }]));
});

test('a private disclosure for another seat does not block or open its identity locally', async () => {
  const h = harness([]);
  assert.equal(await missingCalculationMaterial(h.game,
    [{ type: 'private_open', id: 'secret', viewer: 1, objectId: 7 }], new Set(), 0), null);
  assert.deepEqual(h.state().opened, []);
  assert.ok(await missingCalculationMaterial(h.game,
    [{ type: 'private_open', id: 'secret', viewer: 0, objectId: 7 }], new Set(), 0));
});
