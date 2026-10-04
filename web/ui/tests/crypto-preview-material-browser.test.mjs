import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { previewCryptoRequirementsWithMaterial } from '../src/lib/preview-crypto-material.js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('temporary crypto preview releases its savepoint even if restoration fails', () => {
  const calls = [];
  const game = {
    createRuntimeSavepoint: () => 7,
    injectTranscriptRandomSeeds() { throw new Error('injection rejected'); },
    restoreRuntimeSavepoint(handle) { calls.push(['restore', handle]); throw new Error('restore failed'); },
    releaseRuntimeSavepoint(handle) { calls.push(['release', handle]); },
  };
  assert.throws(() => previewCryptoRequirementsWithMaterial(game, {}, {}), /restore failed/);
  assert.deepEqual(calls, [['restore', 7], ['release', 7]]);
});

test('worker previews seeded mulligan atomically and restores pending randomness on success and failure', { timeout: 120000 }, async t => {
  const vite = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await vite.listen(); t.after(() => vite.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  await page.route('**/crypto-preview-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Atomic crypto preview regression</title>' }));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/crypto-preview-test`);
  const result = await page.evaluate(async () => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
    const decoder = createSnapshotDecoder(), pending = new Map();
    let id = 0;
    const ready = new Promise((resolve, reject) => {
      worker.onerror = event => reject(new Error(event.message));
      worker.onmessage = ({ data }) => {
        if (data.type === 'error') return reject(new Error(data.error.message));
        if (data.type === 'ready') return resolve();
        if (data.type !== 'result') return;
        const request = pending.get(data.id); if (!request) return;
        pending.delete(data.id);
        if (!data.ok) request.reject(new Error(data.error.message));
        else request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
      };
    });
    const call = (method, ...args) => new Promise((resolve, reject) => {
      pending.set(++id, { resolve, reject }); worker.postMessage({ type: 'call', id, method, args });
    });
    worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
    try {
      await ready;
      await call('setPerspective', 0);
      let state = await call('startMatch', {
        playerNames: ['Alice', 'Bob'], startingLife: 20, seed: 1, format: 'normal', startingPlayer: 0,
        openingHandSize: 7, decks: [Array(60).fill('Island'), Array(60).fill('Mountain')],
        publicDecklists: [Array(60).fill('Island'), Array(60).fill('Mountain')],
        hiddenDeckManifests: [0, 1].map(owner => ({ owner, deckCount: 60, commitmentRoot: `ziffle:deck-${owner}`,
          slotCommitments: Array.from({ length: 60 }, (_, slot) => ({ slot, commitment: `ziffle:deck-${owner}:${slot}` })) })),
      });
      const keep = state.decision.actions.find(value => value.action_ref?.kind === 'keep_opening_hand');
      if (!keep) throw new Error('Expected first player keep decision');
      state = await call('dispatch', { type: 'priority_action', action_ref: keep.action_ref });
      const action = state.decision.actions.find(value => value.action_ref?.kind === 'take_mulligan');
      if (!action) throw new Error('Expected normal opening-hand mulligan action');
      const command = { type: 'priority_action', action_ref: action.action_ref };
      const initial = await call('previewCryptoRequirements', command);
      const before = await call('exportPublicAuditCheckpoint');
      const material = { seeds: ['11'.repeat(32)], libraryShuffles: [] };
      // Concurrent audit reads must see restored state. The repeated unseeded
      // preview below also verifies that temporary randomness was removed.
      const [seeded, during, repeated] = await Promise.all([
        call('previewCryptoRequirementsWithMaterial', command, material),
        call('exportPublicAuditCheckpoint'),
        call('previewCryptoRequirementsWithMaterial', command, material),
      ]);
      const after = await call('exportPublicAuditCheckpoint');
      let rejected = false;
      try { await call('previewCryptoRequirementsWithMaterial', { type: 'not_a_command' }, material); }
      catch { rejected = true; }
      const afterFailure = await call('exportPublicAuditCheckpoint');
      const unseededAgain = await call('previewCryptoRequirements', command);
      const savepoint = await call('createRuntimeSavepoint');
      await call('injectTranscriptRandomSeeds', material);
      const explicitlySeeded = await call('previewCryptoRequirements', command);
      await call('restoreRuntimeSavepoint', savepoint);
      await call('releaseRuntimeSavepoint', savepoint);
      // An authoritative command queued behind the preview must produce the
      // same result as that command without the temporary material.
      const dispatchSavepoint = await call('createRuntimeSavepoint');
      await call('dispatch', command);
      const expectedDispatch = await call('exportPublicAuditCheckpoint');
      await call('restoreRuntimeSavepoint', dispatchSavepoint);
      await call('releaseRuntimeSavepoint', dispatchSavepoint);
      await Promise.all([
        call('previewCryptoRequirementsWithMaterial', command, material),
        call('dispatch', command),
      ]);
      const actualDispatch = await call('exportPublicAuditCheckpoint');
      return { initial, seeded, repeated, explicitlySeeded, before, during, after, afterFailure,
        unseededAgain, rejected, expectedDispatch, actualDispatch };
    } finally { worker.terminate(); }
  });
  const opened = requirements => requirements.filter(value => value.type === 'private_open').map(value => value.commitment).sort();
  assert.ok(opened(result.seeded).length, 'preview must contain exact hand openings');
  assert.notDeepEqual(opened(result.seeded), opened(result.initial), 'agreed seed changes the previewed hand');
  assert.deepEqual(result.seeded, result.explicitlySeeded);
  assert.deepEqual(result.repeated, result.seeded);
  assert.deepEqual(result.unseededAgain, result.initial);
  assert.equal(result.rejected, true);
  // Worker timings and snapshot channel revisions change on reads/restores.
  const engineState = ({ __perf, snapshotSerial, ...checkpoint }) => checkpoint;
  for (const checkpoint of [result.during, result.after, result.afterFailure]) assert.deepEqual(engineState(checkpoint), engineState(result.before));
  assert.deepEqual(engineState(result.actualDispatch), engineState(result.expectedDispatch));
});
