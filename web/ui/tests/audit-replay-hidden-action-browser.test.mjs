import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('audit replay hydrates a public land before preview and resolves its stable identity', { timeout: 120000 }, async t => {
  const vite = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await vite.listen(); t.after(() => vite.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  await page.route('**/audit-replay-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Hidden action replay</title>' }));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/audit-replay-test`);
  const result = await page.evaluate(async () => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    const { applyAuditReplayActionWithGame, startAuditTranscriptReplayWithGame } = await import('/src/lib/audit-replay.js');
    const { CURRENT_AUDIT_PROTOCOL_VERSION } = await import('/src/lib/multiplayer-audit.js');
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
    const game = Object.fromEntries(['uiState', 'setPerspective', 'dispatch', 'previewCryptoRequirements', 'revealHiddenPosition',
      'revealHiddenSlot', 'revealHiddenObject', 'getHiddenCardState', 'exportPublicAuditCheckpoint',
      'createRuntimeSavepoint', 'restoreRuntimeSavepoint', 'releaseRuntimeSavepoint']
      .map(method => [method, (...args) => call(method, ...args)]));
    game.startMatch = config => call('startMatch', { ...config, startingPlayer: 0 });
    worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
    try {
      await ready;
      const transcript = { protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION, match: {
        protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
        players: [{ name: 'Alice', deck: Array(60).fill('Island') }, { name: 'Bob', deck: Array(60).fill('Mountain') }],
        openDecklists: true, startingLife: 20, seed: 1, format: 'normal',
        openingHandSize: 7, decks: [[], []],
        hiddenDeckManifests: [0, 1].map(owner => ({ owner, deckCount: 60, commitmentRoot: `root-${owner}`,
          slotCommitments: Array.from({ length: 60 }, (_, slot) => ({ slot, commitment: `commitment-${owner}-${slot}` })) })),
      } };
      let { state } = await startAuditTranscriptReplayWithGame({ game, transcript });
      for (let step = 0; step < 20 && !state.phase.includes('main'); step++) {
        const action = state.decision?.actions?.find(value =>
          ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(value.action_ref?.kind));
        if (!action) throw new Error(`Cannot advance ${JSON.stringify(state.decision)}`);
        state = await call('dispatch', { type: 'priority_action', action_ref: action.action_ref });
      }
      if (!state.phase.includes('main')) throw new Error('Did not reach main phase');
      const checkpoint = await call('getHiddenCardState');
      const card = checkpoint.objects.find(object => object.id === checkpoint.players[0].hand[0]);
      const opening = { owner: 0, slot: card.hiddenCard.slot, commitment: card.hiddenCard.commitment,
        objectId: card.id, card: 'Island', timing: 'pre' };
      const command = { type: 'priority_action', action_ref: { kind: 'play_land', land_id: card.id, back_face: false },
        object_id: card.id, object_stable_id: card.stableId };
      let savepoint = await call('createRuntimeSavepoint');
      await call('revealHiddenSlot', { owner: 0, slot: opening.slot, cardName: 'Island', commitment: opening.commitment, recomputeDecision: true });
      await call('dispatch', command);
      const expected = await call('exportPublicAuditCheckpoint');
      await call('restoreRuntimeSavepoint', savepoint);
      savepoint = await call('createRuntimeSavepoint');
      const first = await applyAuditReplayActionWithGame({ game, action: { seq: 1, command, audit: { openings: [opening] } } });
      const exact = await call('exportPublicAuditCheckpoint');
      await call('restoreRuntimeSavepoint', savepoint);
      // A remote runtime ID can differ; the stable identity is the authority.
      // Never pick a different identical Island merely by its name.
      const remote = { ...command, object_id: card.id + 10000,
        action_ref: { ...command.action_ref, land_id: card.id + 10000 } };
      const second = await applyAuditReplayActionWithGame({ game, action: { seq: 1, command: remote, audit: { openings: [opening] } } });
      const remapped = await call('exportPublicAuditCheckpoint');
      const final = await call('getHiddenCardState');
      
      return { expected, exact, remapped, hashes: [first.publicCheckpointHash, second.publicCheckpointHash],
        opponentHand: final.players[1].hand.map(id => final.objects.find(object => object.id === id).name) };
    } finally { worker.terminate(); }
  });
  const withoutTimings = ({ __perf, ...checkpoint }) => checkpoint;
  assert.deepEqual(withoutTimings(result.exact), withoutTimings(result.expected));
  assert.deepEqual(withoutTimings(result.remapped), withoutTimings(result.expected));
  assert.equal(result.hashes[0], result.hashes[1]);
  assert.ok(result.opponentHand.every(name => name === 'Hidden Card'));
});
