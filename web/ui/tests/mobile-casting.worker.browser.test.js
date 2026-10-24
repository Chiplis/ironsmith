import test from 'node:test';
import assert from 'node:assert/strict';
import { webkit, devices } from 'playwright';
import { createServer } from 'vite';

test('iOS previews, cancels, pays and casts without auxiliary WASM engines', { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent',
    plugins: [{ name: 'reject-analysis-heaps', transform(source, id) {
      if (!id.split('?')[0].endsWith('/src/workers/wasmGameWorker.js')) return;
      return `const NativeWorker = globalThis.Worker;
        globalThis.Worker = class extends NativeWorker {
          constructor(url, options) {
            throw new Error('Unexpected auxiliary engine: ' + String(url) + String(options));
          }
        };\n${source}`;
    } }],
  });
  await vite.listen();
  const browser = await webkit.launch();
  try {
    const page = await browser.newPage(devices['iPhone 13 landscape']);
    await page.route('**/worker-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Casting regression</title>' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/worker-test`);
    const result = await page.evaluate(async () => {
      const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
      const { shouldShareAnalysisRuntime } = await import('/src/lib/analysis-memory-policy.js');
      const decoder = createSnapshotDecoder(), pending = new Map(), errors = [];
      const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
      let id = 0;
      const ready = new Promise((resolve, reject) => {
        worker.onerror = error => reject(new Error(error.message));
        worker.onmessage = ({ data }) => {
          if (data.type === 'error') return reject(new Error(data.error.message));
          if (data.type === 'ready') return resolve(data);
          if (data.type === 'priorityAnalysisError') errors.push(data.error.message);
          if (data.type !== 'result') return;
          const request = pending.get(data.id); if (!request) return;
          pending.delete(data.id);
          if (!data.ok) return request.reject(new Error(data.error.message));
          request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
        };
      });
      const call = (method, ...args) => new Promise((resolve, reject) => {
        pending.set(++id, { resolve, reject }); worker.postMessage({ type: 'call', id, method, args });
      });
      worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/`, sharedAnalysisRuntime: shouldShareAnalysisRuntime() });
      try {
        const capabilities = await ready;
        const snapshotErrors = [];
        for (const method of ['captureExactBuildSnapshot', 'restoreExactBuildSnapshot']) {
          try { await call(method, {}); } catch (error) { snapshotErrors.push(error.message); }
        }
        await call('resetEmpty', ['Alice', 'Bob'], 20);
        for (const name of ['Mountain', 'Forest', 'Forest']) await call('addCardToZone', 0, name, 'battlefield', true);
        for (const name of ['Lightning Bolt', 'Grizzly Bears']) await call('addCardToZone', 0, name, 'hand', true);
        for (const seat of [0, 1]) for (let i = 0; i < 10; i++) await call('addCardToZone', seat, 'Mountain', 'library', true);
        await call('finishPuzzleSetup');
        let state = await call('uiState');
        for (let i = 0; i < 30 && !/first.main/i.test(state.phase); i++) {
          const action = state.decision?.actions?.find(a => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(a.kind));
          if (!action) throw new Error('Cannot advance: ' + JSON.stringify(state.decision));
          state = await call('dispatch', { type: 'priority_action', action_ref: action.action_ref });
        }
        const castAction = (state, name) => state.decision.actions.find(a => a.kind === 'cast_spell'
          && state.players[0].hand_cards.some(c => c.id === a.object_id && c.name === name));
        const bolt = castAction(state, 'Lightning Bolt');
        if (!bolt) throw new Error('No Bolt action');
        const preview = await call('previewCastTargets', [bolt, bolt], 0);
        const afterPreview = await call('uiState');
        const unchanged = afterPreview.players[0].hand_cards.length === state.players[0].hand_cards.length
          && afterPreview.players[0].battlefield.every(c => !c.tapped) && afterPreview.stack_size === 0;
        const payments = [];
        for (const cancel of [true, false]) {
          state = await call('uiState');
          state = await call('dispatch', { type: 'priority_action', action_ref: castAction(state, 'Lightning Bolt').action_ref });
          if (state.decision.kind !== 'targets') throw new Error('Expected targets: ' + state.decision.kind);
          state = await call('dispatch', { type: 'select_targets', targets: [{ kind: 'player', player: 1 }] });
          const payment = state.mana_payment;
          if (!payment) throw new Error('Expected payment: ' + JSON.stringify(state.decision));
          const [options, ranking] = await Promise.all([
            call('getPaymentActivationOptions', payment.request_hash, payment.plan_id), call('analyzePayment'),
          ]);
          payments.push({ options: options != null, ranking: ranking != null, alreadyPlanned: payment.planning_complete });
          if (cancel) {
            state = await call('cancelDecision');
            if (!state.players[0].battlefield.every(c => !c.tapped) || !castAction(state, 'Lightning Bolt')) throw new Error('Cancel did not restore cast');
          } else {
            state = await call('dispatch', { type: 'mana_payment', response: {
              action: 'confirm', plan_id: payment.plan_id, request_hash: payment.request_hash,
            } });
          }
        }
        const stack = state.stack_objects.map(card => card.name);
        for (let i = 0; i < 10 && state.stack_size > 0; i++) {
          const pass = state.decision.actions.find(a => a.kind === 'pass_priority');
          if (!pass) throw new Error('No pass after cast');
          state = await call('dispatch', { type: 'priority_action', action_ref: pass.action_ref });
        }
        // Native savepoints remain usable for undo and verified recovery.
        const savepoint = await call('createRuntimeSavepoint');
        await call('setLife', 1, 5);
        state = await call('restoreRuntimeSavepoint', savepoint);
        return { capabilities, snapshotErrors, preview, unchanged, payments, stack, opponentLife: state.players[1].life, errors };
      } finally { worker.terminate(); }
    });
    assert.ok(result.preview.requirements.length >= 2);
    assert.equal(result.capabilities.exactBuildSnapshots, false);
    assert.equal(result.capabilities.runtimeSavepoints, true);
    assert.equal(result.snapshotErrors.length, 2);
    assert.ok(result.snapshotErrors.every(message => message.includes('unavailable for this runtime')));
    assert.equal(result.unchanged, true);
    assert.ok(result.payments.every(payment => payment.options && (payment.ranking || payment.alreadyPlanned)), JSON.stringify(result));
    assert.ok(result.stack.includes('Lightning Bolt'));
    assert.equal(result.opponentLife, 17);
    assert.deepEqual(result.errors, []);
  } finally { await browser.close(); await vite.close(); }
});
